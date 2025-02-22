use re_components::{EncodedMesh3D, Mesh3D, MeshFormat, RawMesh3D};
use re_renderer::{resource_managers::ResourceLifeTime, RenderContext, Rgba32Unmul};

pub struct LoadedMesh {
    name: String,

    // TODO(andreas): We should only have MeshHandles here (which are generated by the MeshManager!)
    // Can't do that right now because it's too hard to pass the render context through.
    pub mesh_instances: Vec<re_renderer::renderer::MeshInstance>,

    bbox: macaw::BoundingBox,
}

impl LoadedMesh {
    pub fn load(name: String, mesh: &Mesh3D, render_ctx: &RenderContext) -> anyhow::Result<Self> {
        // TODO(emilk): load CpuMesh in background thread.
        match mesh {
            // Mesh from some file format. File passed in bytes.
            Mesh3D::Encoded(encoded_mesh) => {
                Self::load_encoded_mesh(name, encoded_mesh, render_ctx)
            }
            // Mesh from user logging some triangles.
            Mesh3D::Raw(raw_mesh) => Ok(Self::load_raw_mesh(name, raw_mesh, render_ctx)?),
        }
    }

    pub fn load_raw(
        name: String,
        format: MeshFormat,
        bytes: &[u8],
        render_ctx: &RenderContext,
    ) -> anyhow::Result<Self> {
        re_tracing::profile_function!();

        let mesh_instances = match format {
            MeshFormat::Glb | MeshFormat::Gltf => {
                re_renderer::importer::gltf::load_gltf_from_buffer(
                    &name,
                    bytes,
                    ResourceLifeTime::LongLived,
                    render_ctx,
                )
            }
            // TODO(cmc): support obj
            MeshFormat::Obj => anyhow::bail!(".obj files are not supported yet"),
        }?;
        let bbox = re_renderer::importer::calculate_bounding_box(&mesh_instances);

        Ok(Self {
            name,
            bbox,
            mesh_instances,
        })
    }

    fn load_encoded_mesh(
        name: String,
        encoded_mesh: &EncodedMesh3D,
        render_ctx: &RenderContext,
    ) -> anyhow::Result<Self> {
        re_tracing::profile_function!();
        let EncodedMesh3D {
            mesh_id: _,
            format,
            bytes,
            transform,
        } = encoded_mesh;

        let mut slf = Self::load_raw(name, *format, bytes.as_slice(), render_ctx)?;

        // TODO(cmc): Why are we creating the matrix twice here?
        let (scale, rotation, translation) =
            glam::Affine3A::from_cols_array_2d(transform).to_scale_rotation_translation();
        let transform =
            glam::Affine3A::from_scale_rotation_translation(scale, rotation, translation);
        for instance in &mut slf.mesh_instances {
            instance.world_from_mesh = transform * instance.world_from_mesh;
        }
        slf.bbox = re_renderer::importer::calculate_bounding_box(&slf.mesh_instances);

        Ok(slf)
    }

    fn load_raw_mesh(
        name: String,
        raw_mesh: &RawMesh3D,
        render_ctx: &RenderContext,
    ) -> anyhow::Result<Self> {
        re_tracing::profile_function!();

        // TODO(cmc): Having to do all of these data conversions, copies and allocations doesn't
        // really make sense when you consider that both the component and the renderer are native
        // Rust. Need to clean all of that up later.

        let RawMesh3D {
            mesh_id: _,
            vertex_positions,
            vertex_colors,
            vertex_normals,
            indices,
            albedo_factor,
        } = raw_mesh;

        let vertex_positions: &[glam::Vec3] = bytemuck::cast_slice(vertex_positions.as_slice());
        let num_positions = vertex_positions.len();

        let indices = if let Some(indices) = indices {
            indices.clone()
        } else {
            anyhow::ensure!(num_positions % 3 == 0);
            (0..num_positions as u32).collect()
        };
        let num_indices = indices.len();

        let vertex_colors = if let Some(vertex_colors) = vertex_colors {
            vertex_colors
                .iter()
                .map(|c| {
                    Rgba32Unmul::from_rgba_unmul_array(
                        re_types::datatypes::Color::from_u32(*c).to_array(),
                    )
                })
                .collect()
        } else {
            std::iter::repeat(Rgba32Unmul::WHITE)
                .take(num_positions)
                .collect()
        };

        let vertex_normals = if let Some(normals) = vertex_normals {
            normals
                .chunks_exact(3)
                .map(|v| glam::Vec3::from([v[0], v[1], v[2]]))
                .collect::<Vec<_>>()
        } else {
            // TODO(andreas): Calculate normals
            // TODO(cmc): support textured raw meshes
            std::iter::repeat(glam::Vec3::ZERO)
                .take(num_positions)
                .collect()
        };

        let vertex_texcoords = vec![glam::Vec2::ZERO; vertex_normals.len()];

        let bbox = macaw::BoundingBox::from_points(vertex_positions.iter().copied());

        let mesh = re_renderer::mesh::Mesh {
            label: name.clone().into(),
            indices: indices.as_slice().into(),
            vertex_positions: vertex_positions.into(),
            vertex_colors,
            vertex_normals,
            vertex_texcoords,
            materials: smallvec::smallvec![re_renderer::mesh::Material {
                label: name.clone().into(),
                index_range: 0..num_indices as _,
                albedo: render_ctx
                    .texture_manager_2d
                    .white_texture_unorm_handle()
                    .clone(),
                albedo_multiplier: albedo_factor.map_or(re_renderer::Rgba::WHITE, |v| {
                    re_renderer::Rgba::from_rgba_unmultiplied(v.x(), v.y(), v.z(), v.w())
                }),
            }],
        };

        let mesh_instances = vec![re_renderer::renderer::MeshInstance {
            gpu_mesh: render_ctx.mesh_manager.write().create(
                render_ctx,
                &mesh,
                ResourceLifeTime::LongLived,
            )?,
            ..Default::default()
        }];

        Ok(Self {
            name,
            bbox,
            mesh_instances,
        })
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bbox(&self) -> &macaw::BoundingBox {
        &self.bbox
    }
}
