# Third-party source and asset notices

The reference copy identifies [Rezmason/matrix](https://github.com/Rezmason/matrix) as its source. Its [upstream MIT license](https://github.com/Rezmason/matrix/blob/master/LICENSE) is reproduced below; the local copy did not contain the root license or an upstream revision identifier. Preserve this notice when distributing copied or adapted portions. The upstream README says the classic glyphs were derived from an archived official Matrix product; preserve that provenance alongside the MIT notice when distributing `matrixcode_msdf.png`.

## Rezmason/matrix

MIT License

Copyright (c) 2018 Rezmason

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Bundled dependencies

The reference includes third-party libraries and font-generation tools whose notices must be retained if those files are distributed. `reference/matrix/lib/gl-matrix.js` includes the gl-matrix MIT notice; `reference/matrix/lib/holoplaycore.module.js` includes MIT and Looking Glass notices; `reference/matrix/msdfgen/lib/FTL.TXT` contains the FreeType license; `reference/matrix/msdfgen/lib/lodepng.cpp` contains the LodePNG notice, and `tinyxml2.cpp` has its own header notice. These tools and browser libraries are not required for the native renderer merely to load the existing atlas.
