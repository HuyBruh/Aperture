# Community feedback review — 2026-10-30

Source: [the author's Reddit thread](https://www.reddit.com/r/Thermalright/comments/1xb8l0q/).
Comments were read directly after the account holder completed Reddit's challenge.
Reports are user observations, not independently confirmed hardware results.

| Report | Work in this beta | Remaining validation |
| --- | --- | --- |
| Only onboard GPU appears, no way to select another GPU | NVIDIA samples no longer inherit AMD sensor fields | Physical GPU selection and adapter-consistent name/load routing remain open |
| RTX 5070 Ti has no VRAM or fan data | Added NVML memory and fan-percent queries, plus nvidia-smi fallback fields; collection shows fan percent when RPM is unavailable | Requires NVIDIA hardware testing; this machine has AMD |
| GPU hotspot missing | Existing external sensor fallback retained | No invented hotspot reading; exact model/source must be checked |
| Request for 6.86-inch LCD | Support boundary remains explicit | Different hardware/protocol is not validated |
| Installer cannot find executable | Source ZIP warning and portable package already implemented | Check fresh Windows installation |
| CPU temperature missing on Intel / Ryzen 5600 | Existing PawnIO and external sensor paths retained | Model-specific verification remains necessary |
| Theme contributions | Canvas editor and configuration presets added | Portable asset bundles and a contribution guide are future work |

Additional owner requests: process-scoped multi-monitor FPS priority, two more
space-efficient layouts, larger collection labels, reduced outer margins, and a
drag-and-drop custom canvas with image/GIF backgrounds and per-widget readings.

This is a beta, not a claim of vendor-equivalent QA. Local build/tests and native
fixtures cannot establish compatibility with every GPU, driver, USB controller or
LCD firmware. Keep the known-good 4.0 release and personal configuration backed up.
