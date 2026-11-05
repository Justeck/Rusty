"""Guard the real f32 snapshot versus persisted decimal representation boundary."""
import copy
import math
import struct
import unittest
from game_ui12_smoke import same_native_layout


class NativeLayoutComparisonTests(unittest.TestCase):
    def setUp(self):
        self.saved = {"coordinates": {"x": .2, "y": .8, "scale": 1.25}}
        self.native = {"coordinates": {"x": .20000000298023224, "y": .800000011920929, "scale": 1.25}}

    def test_accepts_observed_native_float32_without_requiring_decimal_double_equality(self):
        self.assertNotEqual(self.native, self.saved)
        self.assertTrue(same_native_layout(self.native, self.saved))
        self.assertTrue(same_native_layout({}, {}))

    def test_rejects_even_one_native_float32_step_of_actual_position_change(self):
        changed = copy.deepcopy(self.native)
        bits = struct.unpack("<I", struct.pack("<f", .2))[0]
        changed["coordinates"]["x"] = struct.unpack("<f", struct.pack("<I", bits + 1))[0]
        self.assertFalse(same_native_layout(changed, self.saved))

    def test_rejects_missing_panels_axes_and_unexpected_fields(self):
        for changed in ({}, {"coordinates": {"x": .2, "scale": 1.25}},
                        {"coordinates": {**self.native["coordinates"], "extra": 0}},
                        {**self.native, "unknown": self.native["coordinates"]}):
            self.assertFalse(same_native_layout(changed, self.saved))

    def test_rejects_nonfinite_booleans_and_out_of_bounds_preferences(self):
        for value in (math.nan, math.inf, True, -.1, 1.1, "0.2"):
            changed = copy.deepcopy(self.saved)
            changed["coordinates"]["x"] = value
            self.assertFalse(same_native_layout(self.native, changed))

    def test_rejects_changed_scale_and_native_nonfinite_or_boolean_values(self):
        for axis, value in (("scale", 1.5), ("x", math.nan), ("x", False)):
            changed = copy.deepcopy(self.native)
            changed["coordinates"][axis] = value
            self.assertFalse(same_native_layout(changed, self.saved))


if __name__ == "__main__":
    unittest.main()
