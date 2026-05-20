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

use crate::{Error, Point, point};
use anyhow::Result;

use super::{BitMatrix, GridSampler, SamplerControl};

#[derive(Default)]
pub struct DefaultGridSampler;

impl GridSampler for DefaultGridSampler {
    fn sample_grid(
        &self,
        image: &BitMatrix,
        dimension_x: u32,
        dimension_y: u32,
        controls: &[SamplerControl],
    ) -> Result<BitMatrix> {
        if dimension_x == 0 || dimension_y == 0 {
            return Err(Error::NotFound {
                message: "barcode pattern was not detected".into(),
            }
            .into());
        }

        for SamplerControl { p0, p1, transform } in controls {
            // Precheck the corners of every roi to bail out early if the grid is "obviously" not completely inside the image
            let is_inside = |x: f32, y: f32| {
                transform
                    .transform_point(Point::centered(point(x, y)))
                    .is_some_and(|p| image.is_in(p))
            };
            if !transform.is_valid()
                || !is_inside(p0.x, p0.y)
                || !is_inside(p1.x - 1.0, p0.y)
                || !is_inside(p1.x - 1.0, p1.y - 1.0)
                || !is_inside(p0.x, p1.y - 1.0)
            {
                return Err(Error::NotFound {
                    message: "barcode pattern was not detected".into(),
                }
                .into());
            }
        }

        let mut bits = BitMatrix::new(dimension_x, dimension_y)?;
        for SamplerControl { p0, p1, transform } in controls {
            for y in (p0.y as i32)..(p1.y as i32) {
                for x in (p0.x as i32)..(p1.x as i32) {
                    let p = transform
                        .transform_point(Point::from((x, y)).centered())
                        .ok_or(Error::NotFound {
                            message: "barcode pattern was not detected".into(),
                        })?;

                    // Due to a "numerical instability" in the PerspectiveTransform generation/application it has been observed
                    // that even though all boundary grid points get projected inside the image, it can still happen that an
                    // inner grid points is not. See #563. A true perspective transformation cannot have this property.
                    // The following check takes 100% care of the issue and turned out to be less of a performance impact than feared.
                    // TODO: Check some mathematical/numercial property of mod2_pix to determine if it is a perspective transforation.
                    if !image.is_in(p) {
                        return Err(Error::NotFound {
                            message: "barcode pattern was not detected".into(),
                        }
                        .into());
                    }

                    if image.at_point(p) {
                        bits.set(x as u32, y as u32);
                    }
                }
            }
        }

        Ok(bits)
    }
}
