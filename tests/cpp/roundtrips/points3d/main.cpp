#include <rerun/archetypes/points3d.hpp>
#include <rerun/recording_stream.hpp>

namespace rr = rerun;

int main(int argc, char** argv) {
    auto rec_stream = rr::RecordingStream("rerun_example_roundtrip_points3d");
    rec_stream.save(argv[1]).throw_on_failure();

    rec_stream.log(
        "points3d",
        rr::archetypes::Points3D({{1.0, 2.0, 3.0}, {4.0, 5.0, 6.0}})
            .with_radii({0.42f, 0.43f})
            .with_colors({0xAA0000CC, 0x00BB00DD})
            .with_labels({"hello", "friend"})
            .with_class_ids({126, 127})
            .with_keypoint_ids({2, 3})
            .with_instance_keys({66, 666})
    );
}
