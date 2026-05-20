/*
 * Copyright 2007 ZXing authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::{fmt::Display, str::FromStr};

/**
 * Enumerates barcode formats known to this package. Please keep alphabetized.
 *
 * @author Sean Owen
 */
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum BarcodeFormat {
    /** Aztec 2D barcode format. */
    Aztec,

    /** CODABAR 1D format. */
    Codabar,

    /** Code 39 1D format. */
    Code39,

    /** Code 93 1D format. */
    Code93,

    /** Code 128 1D format. */
    Code128,

    /** Data Matrix 2D barcode format. */
    DataMatrix,

    /** EAN-8 1D format. */
    Ean8,

    /** EAN-13 1D format. */
    Ean13,

    /** ITF (Interleaved Two of Five) 1D format. */
    Itf,

    /** MaxiCode 2D barcode format. */
    Maxicode,

    /** PDF417 format. */
    Pdf417,

    /** QR Code 2D barcode format. */
    QrCode,

    MicroQrCode,

    RectangularMicroQrCode,

    /** RSS 14 */
    Rss14,

    /** RSS EXPANDED */
    RssExpanded,

    /** TELEPEN */
    Telepen,

    /** UPC-A 1D format. */
    UpcA,

    /** UPC-E 1D format. */
    UpcE,

    /** UPC/EAN extension format. Not a stand-alone format. */
    UpcEanExtension,

    DxFilmEdge,

    /// format not supported
    UnsupportedFormat,
}

impl Display for BarcodeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BarcodeFormat::Aztec => "aztec",
                BarcodeFormat::Codabar => "codabar",
                BarcodeFormat::Code39 => "code 39",
                BarcodeFormat::Code93 => "code 93",
                BarcodeFormat::Code128 => "code 128",
                BarcodeFormat::DataMatrix => "datamatrix",
                BarcodeFormat::Ean8 => "ean 8",
                BarcodeFormat::Ean13 => "ean 13",
                BarcodeFormat::Itf => "itf",
                BarcodeFormat::Maxicode => "maxicode",
                BarcodeFormat::Pdf417 => "pdf 417",
                BarcodeFormat::QrCode => "qrcode",
                BarcodeFormat::MicroQrCode => "mqr",
                BarcodeFormat::RectangularMicroQrCode => "rmqr",
                BarcodeFormat::Rss14 => "rss 14",
                BarcodeFormat::RssExpanded => "rss expanded",
                BarcodeFormat::Telepen => "telepen",
                BarcodeFormat::UpcA => "upc a",
                BarcodeFormat::UpcE => "upc e",
                BarcodeFormat::UpcEanExtension => "upc/ean extension",
                BarcodeFormat::DxFilmEdge => "DXFilmEdge",
                _ => "Unsupported",
            }
        )
    }
}

impl From<String> for BarcodeFormat {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&str> for BarcodeFormat {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "aztec" | "aztec code" | "aztec_code" => BarcodeFormat::Aztec,
            "codabar" | "coda" | "coda_bar" | "cod_a_bar" | "cod_a" => BarcodeFormat::Codabar,
            "code 39" | "code_39" | "code39" | "alpha39" | "code_3_of_9" | "uss_39" | "usd-3" => {
                BarcodeFormat::Code39
            }
            "code 93" | "code_93" | "code93" => BarcodeFormat::Code93,
            "code 128" | "code_128" | "code128" | "iso/iec 15417:2007" | "iso/_15417:2007" => {
                BarcodeFormat::Code128
            }
            "datamatrix" | "data matrix" | "data_matrix" => BarcodeFormat::DataMatrix,
            "ean 8" | "ean_8" | "ean8" => BarcodeFormat::Ean8,
            "ean 13" | "ean_13" | "ean13" => BarcodeFormat::Ean13,
            "itf" | "itf_code" | "itf14" | "itf 14" | "itf_14" | "interleaved 2 of 5" => {
                BarcodeFormat::Itf
            }
            "maxicode" | "maxi_code" => BarcodeFormat::Maxicode,
            "pdf 417" | "pdf_417" | "pdf417" | "iso 15438" | "iso_15438" => BarcodeFormat::Pdf417,
            "qrcode" | "qr_code" | "qr code" => BarcodeFormat::QrCode,
            "mqr" | "microqr" | "micro_qr" | "micro_qrcode" | "micro_qr_code" | "mqr_code" => {
                BarcodeFormat::MicroQrCode
            }
            "rmqr" | "rectangular_mqr" | "rectangular_micro_qr" | "rmqr_code" => {
                BarcodeFormat::RectangularMicroQrCode
            }
            "rss 14" | "rss_14" | "rss14" | "gs1 databar" | "gs1 databar coupon"
            | "gs1_databar_coupon" => BarcodeFormat::Rss14,
            "rss expanded" | "expanded rss" | "rss_expanded" => BarcodeFormat::RssExpanded,
            "telepen" => BarcodeFormat::Telepen,
            "upc a" | "upc_a" | "upca" => BarcodeFormat::UpcA,
            "upc e" | "upc_e" | "upce" => BarcodeFormat::UpcE,
            "upc ean extension" | "upc extension" | "ean extension" | "upc/ean extension"
            | "upc_ean_extension" => BarcodeFormat::UpcEanExtension,
            "DXFilmEdge" | "dxfilmedge" | "dx film edge" => BarcodeFormat::DxFilmEdge,
            _ => BarcodeFormat::UnsupportedFormat,
        }
    }
}

impl FromStr for BarcodeFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let format = BarcodeFormat::from(s);
        if format == BarcodeFormat::UnsupportedFormat {
            Err(crate::Error::InvalidFormat {
                message: format!("Unsupported barcode format: {s}"),
            }
            .into())
        } else {
            Ok(format)
        }
    }
}
