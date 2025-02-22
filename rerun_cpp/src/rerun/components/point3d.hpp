// DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/cpp/mod.rs:54.
// Based on "crates/re_types/definitions/rerun/components/point3d.fbs".

#pragma once

#include "../data_cell.hpp"
#include "../datatypes/vec3d.hpp"
#include "../result.hpp"

#include <cstdint>
#include <memory>
#include <utility>

namespace arrow {
    class DataType;
    class FixedSizeListBuilder;
    class MemoryPool;
} // namespace arrow

namespace rerun {
    namespace components {
        /// A point in 3D space.
        struct Point3D {
            rerun::datatypes::Vec3D xyz;

            /// Name of the component, used for serialization.
            static const char* NAME;

          public:
            // Extensions to generated type defined in 'point3d_ext.cpp'

            /// Construct Point3D from x/y/z values.
            Point3D(float x, float y, float z) : xyz{x, y, z} {}

            float x() const {
                return xyz.x();
            }

            float y() const {
                return xyz.y();
            }

            float z() const {
                return xyz.z();
            }

          public:
            Point3D() = default;

            Point3D(rerun::datatypes::Vec3D _xyz) : xyz(std::move(_xyz)) {}

            Point3D& operator=(rerun::datatypes::Vec3D _xyz) {
                xyz = std::move(_xyz);
                return *this;
            }

            Point3D(const float (&arg)[3]) : xyz(arg) {}

            /// Returns the arrow data type this type corresponds to.
            static const std::shared_ptr<arrow::DataType>& arrow_datatype();

            /// Creates a new array builder with an array of this type.
            static Result<std::shared_ptr<arrow::FixedSizeListBuilder>> new_arrow_array_builder(
                arrow::MemoryPool* memory_pool
            );

            /// Fills an arrow array builder with an array of this type.
            static Error fill_arrow_array_builder(
                arrow::FixedSizeListBuilder* builder, const Point3D* elements, size_t num_elements
            );

            /// Creates a Rerun DataCell from an array of Point3D components.
            static Result<rerun::DataCell> to_data_cell(
                const Point3D* instances, size_t num_instances
            );
        };
    } // namespace components
} // namespace rerun
