# Third-Party Notices for KZip

KZip incorporates and dynamically interfaces with third-party open-source software components.
Below are the required attributions and license notices for these components.

---

## 1. 7-Zip & NanaZip Archive Engine (`KZip.Core.dll`, `KZip.Codecs.dll`, `KZip.ZS.Codecs.dll`)

* **7-Zip** - Copyright (C) 1999-2024 Igor Pavlov.
* **NanaZip** - Copyright (C) MouriNaruto (Kenji Mouri) and Contributors.
* **License:** GNU Lesser General Public License v2.1 (LGPL-2.1) with unRAR restriction.

### LGPL-2.1 Compliance Notice
`KZip.Core.dll` and its associated codec libraries (`Codecs\KZip.Codecs.dll`, `Codecs\KZip.ZS.Codecs.dll`) are dynamically linked at runtime via standard COM/Win32 `LoadLibraryW` interfaces. Under the terms of the GNU Lesser General Public License version 2.1, you are free to replace, modify, or relink these dynamic libraries with compatible versions. The complete corresponding source code for the archive engine is maintained in accordance with the LGPL-2.1 license.

---

## 2. Everything SDK 3 (`KZip.Search.dll` / `Everything3.h` / `everything_ipc.h`)

* **Everything SDK** - Copyright (C) 2025 voidtools / David Carpenter.
* **License:** MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

---

## 3. Rust & TypeScript Open-Source Crates

* **Tauri** (Apache-2.0 / MIT) - Copyright (c) Tauri Programme within The Commons Conservancy.
* **ignore & regex** (MIT / Unlicense) - Copyright (c) Andrew Gallant (BurntSushi).
* **libloading** (ISC) - Copyright (c) Simonas Kazlauskas.
