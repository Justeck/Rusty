# Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
"""Acceptance assertions must reject transport errors and unconfirmed damage."""
import unittest
from unittest.mock import Mock
from game_gui_smoke import AcceptanceFailure
from game_schematic_smoke import SchematicSmoke


class SchematicAcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.smoke = object.__new__(SchematicSmoke)

    def test_only_explicit_permission_refusal_is_accepted(self):
        reason = "Construction requires Survival or Creative build permission"
        self.smoke.sch = Mock(side_effect=[
            AcceptanceFailure("Game rejected the rusty-schematic command"),
            {"active": False, "result": reason},
        ])
        self.assertEqual(self.smoke.start_or_permission_refusal(), (True, {"active": False, "result": reason}))

    def test_transport_failure_is_never_a_successful_denial(self):
        self.smoke.sch = Mock(side_effect=AcceptanceFailure("Private control connection or reply failed: TimeoutError"))
        with self.assertRaisesRegex(AcceptanceFailure, "TimeoutError"):
            self.smoke.start_or_permission_refusal()

    def test_remote_refusal_requires_exact_error_and_stopped_permission_status(self):
        remote_error = "Game rejected the rusty-schematic command; see private control error receipt"
        reason = "Construction requires Survival or Creative build permission"
        stopped = {"active": False, "result": reason}
        self.smoke.sch = Mock(side_effect=[AcceptanceFailure(remote_error), stopped])
        self.assertEqual(self.smoke.start_or_permission_refusal(), (True, stopped))
        for error in (
            "Private native control failed: TimeoutError",
            "Private control authentication failed",
            "Native reply was not an object",
            "Game rejected the inventory command; see private control error receipt",
            remote_error + " unexpected suffix",
            "Game rejected the rusty-schematic command; transport failed",
        ):
            with self.subTest(error=error):
                self.smoke.sch = Mock(side_effect=[AcceptanceFailure(error), stopped])
                with self.assertRaises(AcceptanceFailure):
                    self.smoke.start_or_permission_refusal()
                self.assertEqual(self.smoke.sch.call_count, 1, "Unclassified errors must not be converted to a status-based success")
        for status in ({"active": True, "result": reason}, {"active": False, "result": "No schematic loaded"}):
            with self.subTest(status=status):
                self.smoke.sch = Mock(side_effect=[AcceptanceFailure(remote_error), status])
                with self.assertRaises(AcceptanceFailure):
                    self.smoke.start_or_permission_refusal()

    def test_unrelated_game_error_is_never_a_permission_refusal(self):
        self.smoke.sch = Mock(side_effect=[
            AcceptanceFailure("Game rejected the rusty-schematic command"),
            {"active": False, "result": "No schematic loaded"},
        ])
        with self.assertRaises(AcceptanceFailure):
            self.smoke.start_or_permission_refusal()

    def test_damage_requires_a_real_received_event_and_zero_new_uses(self):
        before = {"ordinary_block_uses": 4, "accepted_writes": 0, "damage_receipts": 0}
        good = {"active": False, "ordinary_block_uses": 4, "accepted_writes": 0, "damage_receipts": 1}
        self.smoke.assert_damage_stop(before, good)
        for key, bad in (("damage_receipts", 0), ("ordinary_block_uses", 5), ("accepted_writes", 1), ("active", True)):
            with self.subTest(key=key), self.assertRaises(AcceptanceFailure):
                self.smoke.assert_damage_stop(before, {**good, key: bad})


if __name__ == "__main__":
    unittest.main()
