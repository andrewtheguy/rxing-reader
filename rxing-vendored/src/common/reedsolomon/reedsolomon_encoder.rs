/*
 * Copyright 2008 ZXing authors
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

use crate::Exceptions;
use crate::common::Result;

use super::{GenericGFPoly, GenericGFRef};

/**
 * <p>Implements Reed-Solomon encoding, as the name implies.</p>
 *
 * @author Sean Owen
 * @author William Rucklidge
 */
pub struct ReedSolomonEncoder {
    field: GenericGFRef,
    cached_generators: Vec<GenericGFPoly>,
}

impl ReedSolomonEncoder {
    pub fn new(field: GenericGFRef) -> Result<Self> {
        let n = field;
        Ok(Self {
            cached_generators: vec![GenericGFPoly::new(n, &[1])?],
            field: n,
        })
    }

    fn build_generator(&mut self, degree: usize) -> Result<&GenericGFPoly> {
        if degree >= self.cached_generators.len() {
            let mut last_generator = self
                .cached_generators
                .last()
                .ok_or_else(|| {
                    Exceptions::illegal_state_with(
                        "build_generator: cached_generators is empty",
                    )
                })?
                .clone();
            let cg_len = self.cached_generators.len();
            for d in cg_len..=degree {
                let factor = GenericGFPoly::new(
                    self.field,
                    &[
                        1,
                        self.field.exp(d as i32 - 1 + self.field.get_generator_base()),
                    ],
                )
                .map_err(|e| {
                    Exceptions::illegal_state_with(format!(
                        "build_generator: GenericGFPoly::new failed at degree {d}: {e:?}"
                    ))
                })?;
                let next_generator = last_generator.multiply(&factor).map_err(|e| {
                    Exceptions::illegal_state_with(format!(
                        "build_generator: multiply failed at degree {d}: {e:?}"
                    ))
                })?;
                self.cached_generators.push(next_generator.clone());
                last_generator = next_generator;
            }
        }
        self.cached_generators.get(degree).ok_or_else(|| {
            Exceptions::illegal_state_with(format!(
                "build_generator: missing cached generator for degree {degree}"
            ))
        })
    }

    pub fn encode(&mut self, to_encode: &mut [i32], ec_bytes: usize) -> Result<()> {
        if ec_bytes == 0 {
            return Err(Exceptions::illegal_argument_with(
                "No error correction bytes",
            ));
        }
        if ec_bytes > to_encode.len() {
            return Err(Exceptions::illegal_argument_with(
                "ec_bytes exceeds to_encode length",
            ));
        }
        let data_bytes = to_encode.len() - ec_bytes;
        if data_bytes == 0 {
            return Err(Exceptions::illegal_argument_with("No data bytes provided"));
        }
        let fld = self.field;
        let generator = self.build_generator(ec_bytes)?;
        let mut info_coefficients: Vec<i32> = vec![0; data_bytes];
        info_coefficients[0..data_bytes].clone_from_slice(&to_encode[0..data_bytes]);
        let mut info = GenericGFPoly::new(fld, &info_coefficients)?;
        info = info.multiply_by_monomial(ec_bytes, 1)?;
        let remainder = &info.divide(generator)?.1;
        let coefficients = remainder.get_coefficients();
        let num_zero_coefficients = ec_bytes - coefficients.len();
        for i in 0..num_zero_coefficients {
            to_encode[data_bytes + i] = 0;
        }
        to_encode[data_bytes + num_zero_coefficients
            ..(coefficients.len() + data_bytes + num_zero_coefficients)]
            .clone_from_slice(&coefficients[0..coefficients.len()]);
        Ok(())
    }
}
