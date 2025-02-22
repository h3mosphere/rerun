# DO NOT EDIT!: This file was auto-generated by crates/re_types_builder/src/codegen/python.rs:277.

from __future__ import annotations

from .. import datatypes
from .._baseclasses import (
    BaseDelegatingExtensionArray,
    BaseDelegatingExtensionType,
)

__all__ = ["ColorArray", "ColorType"]


class ColorType(BaseDelegatingExtensionType):
    _TYPE_NAME = "rerun.colorrgba"
    _DELEGATED_EXTENSION_TYPE = datatypes.ColorType


class ColorArray(BaseDelegatingExtensionArray[datatypes.ColorArrayLike]):
    _EXTENSION_NAME = "rerun.colorrgba"
    _EXTENSION_TYPE = ColorType
    _DELEGATED_ARRAY_TYPE = datatypes.ColorArray


ColorType._ARRAY_TYPE = ColorArray

# TODO(cmc): bring back registration to pyarrow once legacy types are gone
# pa.register_extension_type(ColorType())
