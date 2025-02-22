from __future__ import annotations

from collections.abc import Sized
from typing import TYPE_CHECKING, Sequence

import numpy as np
import pyarrow as pa

if TYPE_CHECKING:
    from .. import LineStrip3DArrayLike


def linestrip3d_native_to_pa_array(data: LineStrip3DArrayLike, data_type: pa.DataType) -> pa.Array:
    from ...components import LineStrip3D
    from ...datatypes import Vec3DArray

    # pure-numpy fast path
    if isinstance(data, np.ndarray):
        if len(data) == 0:
            inners = []
        elif data.ndim == 2:
            inners = [Vec3DArray.from_similar(data).storage]
        else:
            o = 0
            offsets = [o] + [o := next_offset(o, arr) for arr in data]
            inner = Vec3DArray.from_similar(data.reshape(-1)).storage
            return pa.ListArray.from_arrays(offsets, inner, type=data_type)

    # pure-object
    elif isinstance(data, LineStrip3D):
        inners = [Vec3DArray.from_similar(data.points).storage]

    # sequences
    elif isinstance(data, Sequence):
        if len(data) == 0:
            inners = []
        elif isinstance(data, np.ndarray):
            inners = [Vec3DArray.from_similar(datum).storage for datum in data]  # type: ignore[union-attr]
        elif isinstance(data[0], LineStrip3D):
            inners = [Vec3DArray.from_similar(datum.points).storage for datum in data]  # type: ignore[union-attr]
        else:
            inners = [Vec3DArray.from_similar(datum).storage for datum in data]  # type: ignore[arg-type]

    else:
        inners = [Vec3DArray.from_similar(data).storage]

    if len(inners) == 0:
        offsets = pa.array([0], type=pa.int32())
        inner = Vec3DArray.from_similar([]).storage
        return pa.ListArray.from_arrays(offsets, inner, type=data_type)

    o = 0
    offsets = [o] + [o := next_offset(o, inner) for inner in inners]

    inner = pa.concat_arrays(inners)

    return pa.ListArray.from_arrays(offsets, inner, type=data_type)


def next_offset(acc: int, arr: Sized) -> int:
    return acc + len(arr)
