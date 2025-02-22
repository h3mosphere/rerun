# DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/python.rs:277.

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Sequence, Union

import numpy as np
import numpy.typing as npt
import pyarrow as pa
from attrs import define, field

from .. import datatypes
from .._baseclasses import (
    BaseExtensionArray,
    BaseExtensionType,
)
from ._overrides import linestrip3d_native_to_pa_array  # noqa: F401

__all__ = ["LineStrip3D", "LineStrip3DArray", "LineStrip3DArrayLike", "LineStrip3DLike", "LineStrip3DType"]


@define
class LineStrip3D:
    r"""
    A line strip in 3D space.

    A line strip is a list of points connected by line segments. It can be used to draw
    approximations of smooth curves.

    The points will be connected in order, like so:
    ```text
           2------3     5
          /        \   /
    0----1          \ /
                     4
    ```
    """

    points: list[datatypes.Vec3D] = field()


if TYPE_CHECKING:
    LineStrip3DLike = Union[LineStrip3D, datatypes.Vec3DArrayLike, npt.NDArray[np.float32]]
else:
    LineStrip3DLike = Any

LineStrip3DArrayLike = Union[LineStrip3D, Sequence[LineStrip3DLike], npt.NDArray[np.float32]]


# --- Arrow support ---


class LineStrip3DType(BaseExtensionType):
    def __init__(self) -> None:
        pa.ExtensionType.__init__(
            self,
            pa.list_(
                pa.field(
                    "item",
                    pa.list_(pa.field("item", pa.float32(), nullable=False, metadata={}), 3),
                    nullable=False,
                    metadata={},
                )
            ),
            "rerun.linestrip3d",
        )


class LineStrip3DArray(BaseExtensionArray[LineStrip3DArrayLike]):
    _EXTENSION_NAME = "rerun.linestrip3d"
    _EXTENSION_TYPE = LineStrip3DType

    @staticmethod
    def _native_to_pa_array(data: LineStrip3DArrayLike, data_type: pa.DataType) -> pa.Array:
        return linestrip3d_native_to_pa_array(data, data_type)


LineStrip3DType._ARRAY_TYPE = LineStrip3DArray

# TODO(cmc): bring back registration to pyarrow once legacy types are gone
# pa.register_extension_type(LineStrip3DType())
