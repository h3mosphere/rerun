// DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/cpp/mod.rs:54.
// Based on "crates/re_types/definitions/rerun/testing/datatypes/fuzzy.fbs".

#pragma once

#include "flattened_scalar.hpp"

#include <cstdint>
#include <memory>
#include <optional>
#include <rerun/result.hpp>
#include <string>
#include <vector>

namespace arrow {
    class DataType;
    class MemoryPool;
    class StructBuilder;
} // namespace arrow

namespace rerun {
    namespace datatypes {
        struct AffixFuzzer1 {
            std::optional<float> single_float_optional;

            std::string single_string_required;

            std::optional<std::string> single_string_optional;

            std::optional<std::vector<float>> many_floats_optional;

            std::vector<std::string> many_strings_required;

            std::optional<std::vector<std::string>> many_strings_optional;

            float flattened_scalar;

            rerun::datatypes::FlattenedScalar almost_flattened_scalar;

            std::optional<bool> from_parent;

          public:
            AffixFuzzer1() = default;

            /// Returns the arrow data type this type corresponds to.
            static const std::shared_ptr<arrow::DataType>& arrow_datatype();

            /// Creates a new array builder with an array of this type.
            static Result<std::shared_ptr<arrow::StructBuilder>> new_arrow_array_builder(
                arrow::MemoryPool* memory_pool
            );

            /// Fills an arrow array builder with an array of this type.
            static Error fill_arrow_array_builder(
                arrow::StructBuilder* builder, const AffixFuzzer1* elements, size_t num_elements
            );
        };
    } // namespace datatypes
} // namespace rerun
