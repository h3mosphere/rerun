//! The Rerun C SDK.
//!
//! The functions here must match `rerun.h`.
// TODO(emilk): error handling

#![crate_type = "staticlib"]
#![allow(clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)] // Too much unsafe

mod error;
mod ptr;

use std::ffi::{c_char, CString};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

use re_sdk::{
    external::re_log_types::{self},
    log::{DataCell, DataRow},
    ComponentName, EntityPath, RecordingStream, RecordingStreamBuilder, StoreKind,
};

// ----------------------------------------------------------------------------
// Types:

type CRecStreamId = u32;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CStoreKind {
    /// A recording of user-data.
    Recording = 1,

    /// Data associated with the blueprint state.
    Blueprint = 2,
}

impl From<CStoreKind> for StoreKind {
    fn from(kind: CStoreKind) -> Self {
        match kind {
            CStoreKind::Recording => StoreKind::Recording,
            CStoreKind::Blueprint => StoreKind::Blueprint,
        }
    }
}

/// Simple C version of [`CStoreInfo`]
#[repr(C)]
#[derive(Debug)]
pub struct CStoreInfo {
    /// The user-chosen name of the application doing the logging.
    pub application_id: *const c_char,

    pub store_kind: CStoreKind,
}

#[repr(C)]
pub struct CDataCell {
    pub component_name: *const c_char,

    /// Length of [`Self::bytes`].
    pub num_bytes: u64,

    /// Data in the Arrow IPC encapsulated message format.
    pub bytes: *const u8,
}

#[repr(C)]
pub struct CDataRow {
    pub entity_path: *const c_char,
    pub num_instances: u32,
    pub num_data_cells: u32,
    pub data_cells: *const CDataCell,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CErrorCode {
    Ok = 0,

    _CategoryArgument = 0x0000_00010,
    UnexpectedNullArgument,
    InvalidStringArgument,
    InvalidRecordingStreamHandle,
    InvalidSocketAddress,
    InvalidEntityPath,

    _CategoryRecordingStream = 0x0000_00100,
    RecordingStreamCreationFailure,
    RecordingStreamSaveFailure,

    _CategoryArrow = 0x0000_1000,
    ArrowIpcMessageParsingFailure,
    ArrowDataCellError,

    Unknown = 0xFFFF_FFFF,
}

#[repr(C)]
#[derive(Clone)]
pub struct CError {
    pub code: CErrorCode,
    pub message: [c_char; 512],
}

// ----------------------------------------------------------------------------
// Global data:

const RERUN_REC_STREAM_CURRENT_RECORDING: CRecStreamId = 0xFFFFFFFF;
const RERUN_REC_STREAM_CURRENT_BLUEPRINT: CRecStreamId = 0xFFFFFFFE;

#[derive(Default)]
pub struct RecStreams {
    next_id: CRecStreamId,
    streams: ahash::HashMap<CRecStreamId, RecordingStream>,
}

impl RecStreams {
    fn insert(&mut self, stream: RecordingStream) -> CRecStreamId {
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        id
    }

    fn get(&self, id: CRecStreamId) -> Option<RecordingStream> {
        match id {
            RERUN_REC_STREAM_CURRENT_RECORDING => RecordingStream::get(StoreKind::Recording, None),
            RERUN_REC_STREAM_CURRENT_BLUEPRINT => RecordingStream::get(StoreKind::Blueprint, None),
            _ => self.streams.get(&id).cloned(),
        }
    }

    fn remove(&mut self, id: CRecStreamId) -> Option<RecordingStream> {
        match id {
            RERUN_REC_STREAM_CURRENT_BLUEPRINT | RERUN_REC_STREAM_CURRENT_RECORDING => None,
            _ => self.streams.remove(&id),
        }
    }
}

/// All recording streams created from C.
static RECORDING_STREAMS: Lazy<Mutex<RecStreams>> = Lazy::new(Mutex::default);

// ----------------------------------------------------------------------------
// Public functions:

// SAFETY: the unsafety comes from #[no_mangle], because we can declare multiple
// functions with the same symbol names, and the linker behavior in this case i undefined.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_version_string() -> *const c_char {
    static VERSION: Lazy<CString> =
        Lazy::new(|| CString::new(re_sdk::build_info().to_string()).unwrap()); // unwrap: there won't be any NUL bytes in the string

    VERSION.as_ptr()
}

#[allow(clippy::result_large_err)]
fn rr_recording_stream_new_impl(store_info: *const CStoreInfo) -> Result<CRecStreamId, CError> {
    initialize_logging();

    let store_info = ptr::try_ptr_as_ref(store_info, "store_info")?;

    let CStoreInfo {
        application_id,
        store_kind,
    } = *store_info;

    let application_id = ptr::try_char_ptr_as_str(application_id, "store_info.application_id")?;

    let mut rec_stream_builder = RecordingStreamBuilder::new(application_id)
        //.is_official_example(is_official_example) // TODO(andreas): Is there a meaningful way to expose this?
        //.store_id(recording_id.clone()) // TODO(andreas): Expose store id.
        .store_source(re_log_types::StoreSource::CSdk);

    if store_kind == CStoreKind::Blueprint {
        rec_stream_builder = rec_stream_builder.blueprint();
    }

    let rec_stream = rec_stream_builder.buffered().map_err(|err| {
        CError::new(
            CErrorCode::RecordingStreamCreationFailure,
            &format!("Failed to create recording stream: {err}"),
        )
    })?;
    Ok(RECORDING_STREAMS.lock().insert(rec_stream))
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_new(
    store_info: *const CStoreInfo,
    error: *mut CError,
) -> CRecStreamId {
    match rr_recording_stream_new_impl(store_info) {
        Err(err) => {
            err.write_error(error);
            0
        }
        Ok(id) => id,
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_free(id: CRecStreamId) {
    if let Some(stream) = RECORDING_STREAMS.lock().remove(id) {
        stream.disconnect();
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_set_global(id: CRecStreamId, store_kind: CStoreKind) {
    let stream = RECORDING_STREAMS.lock().get(id);
    RecordingStream::set_global(store_kind.into(), stream);
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_set_thread_local(id: CRecStreamId, store_kind: CStoreKind) {
    let stream = RECORDING_STREAMS.lock().get(id);
    RecordingStream::set_thread_local(store_kind.into(), stream);
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_flush_blocking(id: CRecStreamId) {
    if let Some(stream) = RECORDING_STREAMS.lock().remove(id) {
        stream.flush_blocking();
    }
}

#[allow(clippy::result_large_err)]
fn rr_recording_stream_connect_impl(
    id: CRecStreamId,
    tcp_addr: *const c_char,
    flush_timeout_sec: f32,
) -> Result<(), CError> {
    let stream = RECORDING_STREAMS
        .lock()
        .get(id)
        .ok_or(CError::invalid_recording_stream_handle())?;

    let tcp_addr = ptr::try_char_ptr_as_str(tcp_addr, "tcp_addr")?;
    let tcp_addr = tcp_addr.parse().map_err(|err| {
        CError::new(
            CErrorCode::InvalidSocketAddress,
            &format!("Failed to parse tcp address {tcp_addr:?}: {err}"),
        )
    })?;

    let flush_timeout = if flush_timeout_sec >= 0.0 {
        Some(std::time::Duration::from_secs_f32(flush_timeout_sec))
    } else {
        None
    };
    stream.connect(tcp_addr, flush_timeout);

    Ok(())
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_connect(
    id: CRecStreamId,
    tcp_addr: *const c_char,
    flush_timeout_sec: f32,
    error: *mut CError,
) {
    if let Err(err) = rr_recording_stream_connect_impl(id, tcp_addr, flush_timeout_sec) {
        err.write_error(error);
    }
}

#[allow(clippy::result_large_err)]
fn rr_recording_stream_save_impl(id: CRecStreamId, path: *const c_char) -> Result<(), CError> {
    let stream = RECORDING_STREAMS
        .lock()
        .get(id)
        .ok_or(CError::invalid_recording_stream_handle())?;

    let path = ptr::try_char_ptr_as_str(path, "path")?;
    stream.save(path).map_err(|err| {
        CError::new(
            CErrorCode::RecordingStreamSaveFailure,
            &format!("Failed to save recording stream to {path:?}: {err}"),
        )
    })
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn rr_recording_stream_save(
    id: CRecStreamId,
    path: *const c_char,
    error: *mut CError,
) {
    if let Err(err) = rr_recording_stream_save_impl(id, path) {
        err.write_error(error);
    }
}

#[allow(unsafe_code)]
#[allow(clippy::result_large_err)]
fn rr_log_impl(
    id: CRecStreamId,
    data_row: *const CDataRow,
    inject_time: bool,
) -> Result<(), CError> {
    let stream = RECORDING_STREAMS
        .lock()
        .get(id)
        .ok_or(CError::invalid_recording_stream_handle())?;

    let data_row = ptr::try_ptr_as_ref(data_row, "data_row")?;

    let CDataRow {
        entity_path,
        num_instances,
        num_data_cells,
        data_cells,
    } = *data_row;

    let entity_path = ptr::try_char_ptr_as_str(entity_path, "entity_path")?;
    let entity_path = match re_log_types::parse_entity_path(entity_path) {
        Ok(entity_path) => EntityPath::from(entity_path),
        Err(err) => {
            return Err(CError::new(
                CErrorCode::InvalidEntityPath,
                &format!("Failed to parse entity path {entity_path:?}: {err}"),
            ))
        }
    };

    re_log::debug!(
        "rerun_log {entity_path:?}, num_instances: {num_instances}, num_data_cells: {num_data_cells}",
    );

    let mut cells = re_log_types::DataCellVec::default();
    cells.reserve(num_data_cells as usize);
    for i in 0..num_data_cells {
        let data_cell: &CDataCell = unsafe { &*data_cells.wrapping_add(i as _) };
        let CDataCell {
            component_name,
            num_bytes,
            bytes,
        } = *data_cell;

        let component_name =
            ptr::try_char_ptr_as_str(component_name, "data_cells[i].component_name")?;
        let component_name = ComponentName::from(component_name);

        let bytes = unsafe { std::slice::from_raw_parts(bytes, num_bytes as usize) };
        let array = parse_arrow_ipc_encapsulated_message(bytes).map_err(|err| {
            CError::new(
                CErrorCode::ArrowIpcMessageParsingFailure,
                &format!("Failed to parse Arrow IPC encapsulated message: {err}"),
            )
        })?;

        cells.push(
            DataCell::try_from_arrow(component_name, array).map_err(|err| {
                CError::new(
                    CErrorCode::ArrowDataCellError,
                    &format!("Failed to create arrow datacell from message: {err}"),
                )
            })?,
        );
    }

    let data_row = DataRow {
        row_id: re_sdk::log::RowId::random(),
        timepoint: Default::default(), // we use the one in the recording stream for now
        entity_path,
        num_instances,
        cells: re_log_types::DataCellRow(cells),
    };

    stream.record_row(data_row, inject_time);

    Ok(())
}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe extern "C" fn rr_log(
    id: CRecStreamId,
    data_row: *const CDataRow,
    inject_time: bool,
    error: *mut CError,
) {
    if let Err(err) = rr_log_impl(id, data_row, inject_time) {
        err.write_error(error);
    }
}

// ----------------------------------------------------------------------------
// Helper functions:

fn initialize_logging() {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(|| {
        re_log::setup_native_logging();
    });
}

fn parse_arrow_ipc_encapsulated_message(
    bytes: &[u8],
) -> Result<Box<dyn arrow2::array::Array>, String> {
    re_log::debug!(
        "parse_arrow_ipc_encapsulated_message: {} bytes",
        bytes.len()
    );

    use arrow2::io::ipc::read::{read_stream_metadata, StreamReader, StreamState};

    let mut cursor = std::io::Cursor::new(bytes);
    let metadata = match read_stream_metadata(&mut cursor) {
        Ok(metadata) => metadata,
        Err(err) => return Err(format!("Failed to read stream metadata: {err}")),
    };
    let stream = StreamReader::new(cursor, metadata, None);
    let chunks: Result<Vec<_>, _> = stream
        .map(|state| match state {
            Ok(StreamState::Some(chunk)) => Ok(chunk),
            Ok(StreamState::Waiting) => {
                unreachable!("cannot be waiting on a fixed buffer")
            }
            Err(err) => Err(err),
        })
        .collect();

    let chunks = chunks.map_err(|err| format!("Arrow error: {err}"))?;

    if chunks.is_empty() {
        return Err("No Chunk found in stream".to_owned());
    }
    if chunks.len() > 1 {
        return Err(format!(
            "Found {} chunks in stream - expected just one.",
            chunks.len()
        ));
    }
    let chunk = chunks.into_iter().next().unwrap();

    let arrays = chunk.into_arrays();

    if arrays.len() != 1 {
        return Err(format!("Expected one array, got {}", arrays.len()));
    }

    Ok(arrays.into_iter().next().unwrap())
}
