package com.klarvo.voice

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/**
 * AC-1/Task 1.3 — Story 11-2: repeatable pause-flush primitive.
 *
 * Guards [KlarvoAudioRecorder.sliceSince], the pure delta-slice function extracted from
 * [KlarvoAudioRecorder.deltaSnapshotWav] specifically so it is JVM-testable without an Android
 * [android.content.Context] (mirrors desktop's `spec_delta_snapshot_disjoint_union`).
 *
 * Scenario: two synthetic sample batches arrive; two `sliceSince` calls (marker advanced between
 * them, as `deltaSnapshotWav()` does) must return DISJOINT slices whose UNION equals the full
 * buffer -- i.e. every sample is transcribed exactly once, never twice, never skipped.
 */
class DeltaSnapshotSliceTest {

    @Test
    fun two_flushes_are_disjoint_and_union_equals_full_buffer() {
        // First batch arrives; marker starts at 0.
        val afterBatch1 = shortArrayOf(1, 2, 3)
        val marker0 = 0
        val delta1 = KlarvoAudioRecorder.sliceSince(afterBatch1, marker0)
        assertArrayEquals(shortArrayOf(1, 2, 3), delta1)

        // deltaSnapshotWav() advances the marker to the buffer size AT FLUSH TIME.
        val marker1 = afterBatch1.size // 3

        // Second batch of samples arrives, appended to the same growing buffer.
        val afterBatch2 = shortArrayOf(1, 2, 3, 4, 5)
        val delta2 = KlarvoAudioRecorder.sliceSince(afterBatch2, marker1)
        assertArrayEquals(shortArrayOf(4, 5), delta2)

        // Disjoint: no sample index appears in both deltas.
        val delta1Set = delta1.toSet()
        val delta2Set = delta2.toSet()
        assert(delta1Set.intersect(delta2Set).isEmpty()) {
            "delta1 and delta2 must be disjoint -- got overlap: ${delta1Set.intersect(delta2Set)}"
        }

        // Union: concatenation of both deltas (in order) reconstructs the full buffer.
        val union = delta1 + delta2
        assertArrayEquals(afterBatch2, union)
    }

    /**
     * Inversion (per DoD): skip advancing the marker between flushes -- the second delta then
     * OVERLAPS the first (re-transcribes samples already flushed), proving the marker-advance
     * step is load-bearing, not redundant. Documented empirically per story DoD.
     */
    @Test
    fun skippingMarkerAdvance_causesOverlap_provingMarkerAdvanceIsLoadBearing() {
        val afterBatch1 = shortArrayOf(1, 2, 3)
        val delta1 = KlarvoAudioRecorder.sliceSince(afterBatch1, 0)

        val afterBatch2 = shortArrayOf(1, 2, 3, 4, 5)
        // BUG under test: re-using marker=0 instead of the advanced marker (3).
        val deltaWithStaleMarker = KlarvoAudioRecorder.sliceSince(afterBatch2, 0)

        // With the marker NOT advanced, the "second" delta re-includes delta1's samples --
        // it is NOT equal to the correct disjoint delta (4,5), and it overlaps delta1.
        assertNotEquals(
            "Stale marker must overlap the first delta (RED case) -- if this ever equals the " +
                "correct disjoint delta, the marker-advance step has silently become a no-op",
            listOf<Short>(4, 5),
            deltaWithStaleMarker.toList()
        )
        val overlap = delta1.toSet().intersect(deltaWithStaleMarker.toSet())
        assert(overlap.isNotEmpty()) {
            "Expected the stale-marker delta to overlap delta1 (that's the bug this test proves " +
                "the real marker-advance path avoids), but got no overlap: $overlap"
        }
    }
}
