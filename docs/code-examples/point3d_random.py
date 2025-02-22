"""Log some random points with color and radii."""
import rerun as rr
from numpy.random import default_rng

rr.init("rerun_example_points", spawn=True)
rng = default_rng(12345)

positions = rng.uniform(-5, 5, size=[10, 3])
colors = rng.uniform(0, 255, size=[10, 3])
radii = rng.uniform(0, 1, size=[10])

rr.log_points("random", positions=positions, colors=colors, radii=radii)
