# ani-cli-rs 0.11.3

## Fixed

- Restored audio for HLS streams that expose audio as a separate rendition.
  ani-cli-rs now retains the source master playlist instead of selecting a
  video-only quality playlist, allowing the local relay and player to load the
  associated audio track.
- Applied the fix to both the Anikoto API/MegaPlay and Anikoto.cz provider
  paths.

## Verification

- Added regression coverage for HLS masters with separate audio renditions.
- Ran formatting, compilation, and the full test suite.
