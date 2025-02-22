# DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/python.rs:382.

from __future__ import annotations

from .annotation_context import (
    AnnotationContext,
    AnnotationContextArray,
    AnnotationContextArrayLike,
    AnnotationContextLike,
    AnnotationContextType,
)
from .class_id import ClassIdArray, ClassIdType
from .color import ColorArray, ColorType
from .disconnected_space import (
    DisconnectedSpace,
    DisconnectedSpaceArray,
    DisconnectedSpaceArrayLike,
    DisconnectedSpaceLike,
    DisconnectedSpaceType,
)
from .draw_order import DrawOrder, DrawOrderArray, DrawOrderArrayLike, DrawOrderLike, DrawOrderType
from .fuzzy import (
    AffixFuzzer1Array,
    AffixFuzzer1Type,
    AffixFuzzer2Array,
    AffixFuzzer2Type,
    AffixFuzzer3Array,
    AffixFuzzer3Type,
    AffixFuzzer4Array,
    AffixFuzzer4Type,
    AffixFuzzer5Array,
    AffixFuzzer5Type,
    AffixFuzzer6Array,
    AffixFuzzer6Type,
    AffixFuzzer7,
    AffixFuzzer7Array,
    AffixFuzzer7ArrayLike,
    AffixFuzzer7Like,
    AffixFuzzer7Type,
    AffixFuzzer8,
    AffixFuzzer8Array,
    AffixFuzzer8ArrayLike,
    AffixFuzzer8Like,
    AffixFuzzer8Type,
    AffixFuzzer9,
    AffixFuzzer9Array,
    AffixFuzzer9ArrayLike,
    AffixFuzzer9Like,
    AffixFuzzer9Type,
    AffixFuzzer10,
    AffixFuzzer10Array,
    AffixFuzzer10ArrayLike,
    AffixFuzzer10Like,
    AffixFuzzer10Type,
    AffixFuzzer11,
    AffixFuzzer11Array,
    AffixFuzzer11ArrayLike,
    AffixFuzzer11Like,
    AffixFuzzer11Type,
    AffixFuzzer12,
    AffixFuzzer12Array,
    AffixFuzzer12ArrayLike,
    AffixFuzzer12Like,
    AffixFuzzer12Type,
    AffixFuzzer13,
    AffixFuzzer13Array,
    AffixFuzzer13ArrayLike,
    AffixFuzzer13Like,
    AffixFuzzer13Type,
    AffixFuzzer14Array,
    AffixFuzzer14Type,
    AffixFuzzer15Array,
    AffixFuzzer15Type,
    AffixFuzzer16,
    AffixFuzzer16Array,
    AffixFuzzer16ArrayLike,
    AffixFuzzer16Like,
    AffixFuzzer16Type,
    AffixFuzzer17,
    AffixFuzzer17Array,
    AffixFuzzer17ArrayLike,
    AffixFuzzer17Like,
    AffixFuzzer17Type,
    AffixFuzzer18,
    AffixFuzzer18Array,
    AffixFuzzer18ArrayLike,
    AffixFuzzer18Like,
    AffixFuzzer18Type,
    AffixFuzzer19Array,
    AffixFuzzer19Type,
    AffixFuzzer20Array,
    AffixFuzzer20Type,
)
from .instance_key import InstanceKey, InstanceKeyArray, InstanceKeyArrayLike, InstanceKeyLike, InstanceKeyType
from .keypoint_id import KeypointIdArray, KeypointIdType
from .label import LabelArray, LabelType
from .line_strip2d import LineStrip2D, LineStrip2DArray, LineStrip2DArrayLike, LineStrip2DLike, LineStrip2DType
from .line_strip3d import LineStrip3D, LineStrip3DArray, LineStrip3DArrayLike, LineStrip3DLike, LineStrip3DType
from .origin3d import Origin3DArray, Origin3DType
from .point2d import Point2DArray, Point2DType
from .point3d import Point3DArray, Point3DType
from .radius import Radius, RadiusArray, RadiusArrayLike, RadiusLike, RadiusType
from .tensor_data import TensorDataArray, TensorDataType
from .transform3d import Transform3DArray, Transform3DType
from .vector3d import Vector3DArray, Vector3DType

__all__ = [
    "AffixFuzzer10",
    "AffixFuzzer10Array",
    "AffixFuzzer10ArrayLike",
    "AffixFuzzer10Like",
    "AffixFuzzer10Type",
    "AffixFuzzer11",
    "AffixFuzzer11Array",
    "AffixFuzzer11ArrayLike",
    "AffixFuzzer11Like",
    "AffixFuzzer11Type",
    "AffixFuzzer12",
    "AffixFuzzer12Array",
    "AffixFuzzer12ArrayLike",
    "AffixFuzzer12Like",
    "AffixFuzzer12Type",
    "AffixFuzzer13",
    "AffixFuzzer13Array",
    "AffixFuzzer13ArrayLike",
    "AffixFuzzer13Like",
    "AffixFuzzer13Type",
    "AffixFuzzer14Array",
    "AffixFuzzer14Type",
    "AffixFuzzer15Array",
    "AffixFuzzer15Type",
    "AffixFuzzer16",
    "AffixFuzzer16Array",
    "AffixFuzzer16ArrayLike",
    "AffixFuzzer16Like",
    "AffixFuzzer16Type",
    "AffixFuzzer17",
    "AffixFuzzer17Array",
    "AffixFuzzer17ArrayLike",
    "AffixFuzzer17Like",
    "AffixFuzzer17Type",
    "AffixFuzzer18",
    "AffixFuzzer18Array",
    "AffixFuzzer18ArrayLike",
    "AffixFuzzer18Like",
    "AffixFuzzer18Type",
    "AffixFuzzer19Array",
    "AffixFuzzer19Type",
    "AffixFuzzer1Array",
    "AffixFuzzer1Type",
    "AffixFuzzer20Array",
    "AffixFuzzer20Type",
    "AffixFuzzer2Array",
    "AffixFuzzer2Type",
    "AffixFuzzer3Array",
    "AffixFuzzer3Type",
    "AffixFuzzer4Array",
    "AffixFuzzer4Type",
    "AffixFuzzer5Array",
    "AffixFuzzer5Type",
    "AffixFuzzer6Array",
    "AffixFuzzer6Type",
    "AffixFuzzer7",
    "AffixFuzzer7Array",
    "AffixFuzzer7ArrayLike",
    "AffixFuzzer7Like",
    "AffixFuzzer7Type",
    "AffixFuzzer8",
    "AffixFuzzer8Array",
    "AffixFuzzer8ArrayLike",
    "AffixFuzzer8Like",
    "AffixFuzzer8Type",
    "AffixFuzzer9",
    "AffixFuzzer9Array",
    "AffixFuzzer9ArrayLike",
    "AffixFuzzer9Like",
    "AffixFuzzer9Type",
    "AnnotationContext",
    "AnnotationContextArray",
    "AnnotationContextArrayLike",
    "AnnotationContextLike",
    "AnnotationContextType",
    "ClassIdArray",
    "ClassIdType",
    "ColorArray",
    "ColorType",
    "DisconnectedSpace",
    "DisconnectedSpaceArray",
    "DisconnectedSpaceArrayLike",
    "DisconnectedSpaceLike",
    "DisconnectedSpaceType",
    "DrawOrder",
    "DrawOrderArray",
    "DrawOrderArrayLike",
    "DrawOrderLike",
    "DrawOrderType",
    "InstanceKey",
    "InstanceKeyArray",
    "InstanceKeyArrayLike",
    "InstanceKeyLike",
    "InstanceKeyType",
    "KeypointIdArray",
    "KeypointIdType",
    "LabelArray",
    "LabelType",
    "LineStrip2D",
    "LineStrip2DArray",
    "LineStrip2DArrayLike",
    "LineStrip2DLike",
    "LineStrip2DType",
    "LineStrip3D",
    "LineStrip3DArray",
    "LineStrip3DArrayLike",
    "LineStrip3DLike",
    "LineStrip3DType",
    "Origin3DArray",
    "Origin3DType",
    "Point2DArray",
    "Point2DType",
    "Point3DArray",
    "Point3DType",
    "Radius",
    "RadiusArray",
    "RadiusArrayLike",
    "RadiusLike",
    "RadiusType",
    "TensorDataArray",
    "TensorDataType",
    "Transform3DArray",
    "Transform3DType",
    "Vector3DArray",
    "Vector3DType",
]
