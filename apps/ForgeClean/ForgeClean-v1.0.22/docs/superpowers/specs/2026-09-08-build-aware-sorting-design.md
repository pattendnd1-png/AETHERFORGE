# ForgeClean v0.5.2 Build-Aware Sorting Design

## Goal

ForgeClean must organize Downloads by project/build identity before generic file type. Files that belong to the same project version must remain together in a canonical build folder and remain directly usable by build/install/verify scripts after relocation.

## Canonical layout

```text
~/Downloads/ForgeClean/Projects/<Project>/
├── Active/
├── Builds/
│   └── <Version>/
├── Downloads/
├── Releases/
├── Restored/
└── Cold/
```

`Builds/<Version>/` contains the complete live build bundle: source archives, HIT-IT/install/build scripts, checksum files, verification logs, package artifacts, and other filenames that carry the same project/version identity.

## Identity rules

1. Detect project/version from a basename before generic extension classification.
2. Recognize semantic-style version tokens after a project prefix, including `-v0.5.2-`, `-10.0.30-`, and underscore variants such as `-V10_2_66-`.
3. The project is the filename prefix before the version token, sanitized for a directory component.
4. The canonical version directory preserves whether the source token used `v`, while normalizing uppercase `V` to `v` and underscores to dots.
5. A project tree remains an `Active` source candidate and is not treated as a build artifact.
6. A project-known file without an explicit version continues to `Projects/<Project>/Downloads/` rather than being guessed into a version.
7. Only entries with no reliable project/build identity use generic type folders.

## Live build bundle policy

Build artifacts are moved, not ColdPacked immediately. They must stay live because sibling scripts may require them. Old build bundles can be cold-archived by an explicit/retention workflow later; v0.5.2 does not invent a new automatic build-retention policy.

## Compatibility rerouting

After a build artifact is moved, ForgeClean creates a guarded symlink at its original `~/Downloads/<filename>` path pointing to the canonical `Builds/<Version>/<filename>` file. This preserves existing one-line commands and relative sibling references because every moved member of a build bundle receives the same compatibility alias.

ForgeClean never overwrites a real file or a conflicting symlink. Existing correct symlinks are accepted idempotently.

## Script classification

Standalone `.sh`, `.bash`, `.fish`, `.run`, and executable project scripts are recognized as installer/build-script material for fallback classification, but a reliable project/version identity always wins over file type.

## Safety

- Never overwrite a destination.
- Never overwrite a real legacy path.
- Never follow top-level symlink aliases back into the organizer.
- Never immediately ColdPack a recognized build artifact.
- Preserve current Active-project, ColdPack, GC, package-cleanup, and external-offload behavior.
- Preserve the v0.4.2+ guarantee that HIT-IT always produces a verification TXT.

## Verification

Host gates must prove:

- multiple ForgeClean v0.5.2 artifacts converge into one `Projects/ForgeClean/Builds/v0.5.2/` directory;
- a second AetherForge-style name such as `ForgeHX-10.0.30-VERIFY.txt` routes to `Projects/ForgeHX/Builds/10.0.30/`;
- original top-level paths become symlinks to canonical build files;
- build files remain live and are not immediately ColdPacked;
- unassociated generic files retain type-based ColdPack behavior;
- existing Active project activation/build redirection remains green.
