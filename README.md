# PdfRust

Pdf parser in pure Rust.

References : https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.7old.pdf.

## Installation

```sh
cargo install pdfrust
```

## Usage

Text
```zsh
pdfrust <pdf_file>.pdf
```

Text characters in tabular format
```zsh
pdfrust --chars <pdf_file>.pdf
```

Page raw content stream
```sh
pdfrust --raw-content <pdf_file>.pdf
```

Fonts 
```sh
pdfrust --font <pdf_file>.pdf
```

## Contributions

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are greatly appreciated.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement". Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (git checkout -b feature/AmazingFeature)
3. Commit your Changes (git commit -m 'Add some AmazingFeature')
4. Push to the Branch (git push origin feature/AmazingFeature)
5. Open a Pull Request

Make sure that unit and integration tests pass.

```sh
cargo test
```

## License

Distributed under the GNU GPL v3.0 License. See LICENSE.txt for more information.
