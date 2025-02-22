#include <rerun/archetypes/line_strips3d.hpp>
#include <rerun/recording_stream.hpp>

namespace rr = rerun;

int main(int argc, char** argv) {
    auto rec_stream = rr::RecordingStream("rerun_example_roundtrip_line_strip3d");
    rec_stream.save(argv[1]).throw_on_failure();

    rec_stream.log(
        "line_strips3d",
        rr::archetypes::LineStrips3D(
            {
                rr::components::LineStrip3D({{0.f, 0.f, 0.f}, {2.f, 1.f, -1.f}}),
                rr::components::LineStrip3D({{4.f, -1.f, 3.f}, {6.f, 0.f, 1.5f}}),
            }
        )
            .with_radii({0.42f, 0.43f})
            .with_colors({0xAA0000CC, 0x00BB00DD})
            .with_labels({"hello", "friend"})
            .with_class_ids({126, 127})
            .with_instance_keys({66, 666})
    );
}
