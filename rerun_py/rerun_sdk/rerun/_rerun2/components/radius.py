# DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/python.rs:277.

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Sequence, Union

import numpy as np
import numpy.typing as npt
import pyarrow as pa
from attrs import define, field

from .._baseclasses import (
    BaseExtensionArray,
    BaseExtensionType,
)
from ._overrides import radius_native_to_pa_array  # noqa: F401

__all__ = ["Radius", "RadiusArray", "RadiusArrayLike", "RadiusLike", "RadiusType"]


@define
class Radius:
    """A Radius component."""

    value: float = field(converter=float)

    def __array__(self, dtype: npt.DTypeLike = None) -> npt.NDArray[Any]:
        return np.asarray(self.value, dtype=dtype)

    def __float__(self) -> float:
        return float(self.value)


if TYPE_CHECKING:
    RadiusLike = Union[Radius, float]
else:
    RadiusLike = Any

RadiusArrayLike = Union[Radius, Sequence[RadiusLike], float, npt.NDArray[np.float32]]


# --- Arrow support ---


class RadiusType(BaseExtensionType):
    def __init__(self) -> None:
        pa.ExtensionType.__init__(self, pa.float32(), "rerun.radius")


class RadiusArray(BaseExtensionArray[RadiusArrayLike]):
    _EXTENSION_NAME = "rerun.radius"
    _EXTENSION_TYPE = RadiusType

    @staticmethod
    def _native_to_pa_array(data: RadiusArrayLike, data_type: pa.DataType) -> pa.Array:
        return radius_native_to_pa_array(data, data_type)


RadiusType._ARRAY_TYPE = RadiusArray

# TODO(cmc): bring back registration to pyarrow once legacy types are gone
# pa.register_extension_type(RadiusType())
