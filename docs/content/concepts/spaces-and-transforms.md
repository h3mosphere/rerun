---
title: Spaces and Transforms
order: 2
---

## The Definition of Spaces

Every Entity in Rerun exists in some *Space*. This is at the core of how Rerun organizes the visualizations of the data
that you have logged.  In the [Rerun Viewer](../reference/viewer.md) you view data by configuring a "Space View", which is a view
of a set of entities *as seen from a particular Space.*

A "Space" is, very loosely, a generalization of the idea of a "Coordinate System" (sometimes known as a "Coordinate Frame") to arbitrary data. If a collection of
entities are part of the same Space, it means they can be rendered together in the same "coordinate system".

For examples:
- For 2d and 3d geometric primitives this means they share the same origin and coordinate system.
- For scalar plots it means they share the same plot axes.
- For text logs, it means they share the same conceptual stream.

As explained bellow, a Space View *may* display data belonging to multiple Spaces, but its coordinate system is defined
by a specific Space, and the other Spaces must have well-defined transforms to that Space to be displayed in the same view.

Which entities belong to which Spaces is a function of the Transform system, which uses the following rules to define
the connectivity of Spaces:

1.  Every unique Entity Path defines a potentially unique space.
1.  Unless otherwise specified, every path is trivially connected to its parent by the Identity transform.
1.  Logging a transform to a path defines the relationship between that path and its parent (replacing the Identity
    connection).
1.  Only paths which are connected by the Identity transform are effectively considered to be part of the same
    Space. All others are considered to be disjoint.

Note that in the absence of transforms, all entity paths are fully connected by the Identity transform, and therefore
share the same Space. However, as soon as you begin to log transforms, you can end up with additional spaces.

Consider the following scenario:

```python
rr.log_points("world/mapped_keypoints", …)
rr.log_points("world/robot/observed_features", …)
rr.log_transform3d("world/robot", …)
```

There are 4 parent/child entity relationships represented in this hierarchy.

- `(root)` -> `world`
- `world` -> `world/mapped_keypoints`
- `world` -> `world/robot`
- `world/robot` -> `world/robot/observed_features`

The call: `rr.log_transform3d("world/robot", …)` only applies to the relationship: `world` -> `world/robot` because the
logged transform (`world/robot`) describes the relationship between the entity and its _parent_ (`world`). All other
relationships are considered to be an identity transform.

This leaves us with two spaces. In one space, we have the entities `world`, and `world/mapped_keypoints`. In the other
space we have the entities `world/robot` and `world/robot/observed_features`.

Practically speaking, this means that the position values of the points from `world/mapped_keypoints` and the points
from `world/robot/observed_features` are not directly comparable. If you were to directly draw these points in a single
coordinate system the results would be meaningless. As noted above, Rerun can still display these entities in the same
Space View because it is able to automatically transform data between different spaces.


## Space Transformations

In order to correctly display data from different Spaces in the same view, Rerun uses the information from logged
transforms. Since most transforms are invertible, Rerun can usually transform data from a parent space to a child space
or vice versa.  As long as there is a continuous chain of well-defined transforms, Rerun will apply the correct series
of transformations to the component data when building the scene.

Rerun transforms are currently limited to connections between _Spatial_ views of 2D or 3D data. There are 3 types of
transforms that can be logged:

- Affine 3D transforms, which can define any combination of translation, rotation, and scale relationship between two paths.
  [rerun.log_transform3d](https://ref.rerun.io/docs/python/latest/common/transforms/#rerun.log_transform3d))
- Pinhole transforms define a 3D -> 2D camera projection. (See:
  [rerun.log_pinhole](https://ref.rerun.io/docs/python/latest/common/transforms/#rerun.log_pinhole))
- A disconnected space specifies that the data cannot be transformed. In this case it will not be possible to combine the
  data into a single view, and you will need to create two separate views to explore the data. (See:
  [rerun.log_disconnected_space](https://ref.rerun.io/docs/python/latest/common/transforms/#rerun.log_disconnected_space))

In the future, Rerun will be adding support for additional types of transforms.
 - [#349: Log 2D -> 2D transformations in the transform hierarchy](https://github.com/rerun-io/rerun/issues/349)


## Examples

Say you have a 3D world with two cameras with known extrinsics (pose) and intrinsics (pinhole model and resolution). You want to log some things in the shared 3D space, and also log each camera image and some detection in these images.

```py
# Log some data to the 3D world:
rr.log_points("world/points", …)

# Log first camera:
rr.log_transform3d("world/camera/#0", rr.TranslationAndMat3(translation=cam0_pose.pos, matrix=cam0_pose.rot))
rr.log_pinhole("world/camera/#0/image", …)

# Log second camera:
rr.log_transform3d("world/camera/#1", rr.TranslationAndMat3(translation=cam1_pose.pos, matrix=cam1_pose.rot))
rr.log_pinhole("world/camera/#1/image", …)

# Log some data to the image spaces of the first camera:
rr.log_image("world/camera/#0/image", …)
rr.log_rect("world/camera/#0/image/detection", …)
```

Rerun will from this understand how the `world` space and the two image spaces (`world/camera/#0/image` and `world/camera/#1/image`) relate to each other, which allows you to explore their relationship in the Rerun Viewer. In the 3D view you will see the two cameras show up with their respective camera frustums (based on the intrinsics). If you hover your mouse in one of the image spaces, a corresponding ray will be shot through the 3D space.

Note that none of the names in the paths are special.


## View coordinates
You can use [`rr.log_view_coordinates`](https://ref.rerun.io/docs/python/latest/common/transforms/#rerun.log_view_coordinates) to set your preferred view coordinate systems, giving semantic meaning to the XYZ axes of the space.

For 3D spaces it can be used to log what the up-axis is in your coordinate system. This will help Rerun set a good default view of your 3D scene, as well as make the virtual eye interactions more natural. This can be done with `rr.log_view_coordinates("world", up="+Z", timeless=True)`.

You can also use this `log_view_coordinates` for pinhole entities, but it is encouraged that you instead use `rr.log_pinhole(…, camera_xyz=)`](https://ref.rerun.io/docs/python/latest/common/transforms/#rerun.log_pinhole) for this. The default coordinate system for pinhole entities is `RDF` (X=Right, Y=Down, Z=Forward).

WARNING: unlike in 3D views where `log_view_coordinates` only impacts how the rendered scene is oriented, applying `log_view_coordinates` to a pinhole-camera will actually influence the projection transform chain. Under the hood this value inserts a hidden transform that re-orients the axis of projection. Different world-content will be projected into your camera with different orientations depending on how you choose this value. See for instance the `open_photogrammetry_format` example.

For 2D spaces and other entities the view coordinates currently do nothing (https://github.com/rerun-io/rerun/issues/1387).
