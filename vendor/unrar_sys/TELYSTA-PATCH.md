Based on crates.io unrar_sys 0.5.8. Original source and license are retained.

Local changes:
- Detect the Cargo target in build.rs, rather than the build host.
- Compile isnt.cpp for Windows cross-builds.
- Link required Windows libraries and statically link GNU winpthreads.
- Use lowercase Windows header names, supported by both MSVC and MinGW on
  case-sensitive filesystems.

These changes make the same project build natively on Windows and cross-build
from Linux, without requiring any runtime UnRAR DLL.
