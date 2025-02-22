"""Log a batch of 3d line strips."""
import rerun as rr

rr.init("rerun_example_line_strip3d", spawn=True)

rr.log_line_strips_3d(
    "batch",
    [
        [
            [0, 0, 2],
            [1, 0, 2],
            [1, 1, 2],
            [0, 1, 2],
        ],
        [
            [0, 0, 0],
            [0, 0, 1],
            [1, 0, 0],
            [1, 0, 1],
            [1, 1, 0],
            [1, 1, 1],
            [0, 1, 0],
            [0, 1, 1],
        ],
    ],
    colors=[[255, 0, 0], [0, 255, 0]],
    stroke_widths=[0.05, 0.01],
)
