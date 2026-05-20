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


use crate::Exceptions;
use crate::common::Result;

use super::{GenericGF, GenericGFPoly, GenericGFRef};

/**
 * <p>Implements Reed-Solomon decoding, as the name implies.</p>
 *
 * <p>The algorithm will not be explained here, but the following references were helpful
 * in creating this implementation:</p>
 *
 * <ul>
 * <li>Bruce Maggs.
 * <a href="http://www.cs.cmu.edu/afs/cs.cmu.edu/project/pscico-guyb/realworld/www/rs_decode.ps">
 * "Decoding Reed-Solomon Codes"</a> (see discussion of Forney's Formula)</li>
 * <li>J.I. Hall. <a href="www.mth.msu.edu/~jhall/classes/codenotes/GRS.pdf">
 * "Chapter 5. Generalized Reed-Solomon Codes"</a>
 * (see discussion of Euclidean algorithm)</li>
 * </ul>
 *
 * <p>Much credit is due to William Rucklidge since portions of this code are an indirect
 * port of his C++ Reed-Solomon implementation.</p>
 *
 * @author Sean Owen
 * @author William Rucklidge
 * @author sanfordsquires
 */
pub struct ReedSolomonDecoder {
    field: GenericGFRef,
}

impl ReedSolomonDecoder {
    pub const fn new(field: GenericGFRef) -> Self {
        Self { field }
    }

    /**
     * <p>Decodes given set of received codewords, which include both data and error-correction
     * codewords. Really, this means it uses Reed-Solomon to detect and correct errors, in-place,
     * in the input.</p>
     *
     * @param received data and error-correction codewords
     * @param two_s number of error-correction codewords available
     * @throws ReedSolomonException if decoding fails for any reason
     */
    pub fn decode(&self, received: &mut [i32], two_s: i32) -> Result<usize> {
        let poly = GenericGFPoly::new(self.field, received)?;
        let mut syndrome_coefficients = vec![0; two_s as usize];
        let mut no_error = true;
        for i in 0..two_s {
            let eval = poly.evaluate_at(self.field.exp(i + self.field.get_generator_base()) as usize);
            let len = syndrome_coefficients.len();
            syndrome_coefficients[len - 1 - i as usize] = eval;
            if eval != 0 {
                no_error = false;
            }
        }
        if no_error {
            return Ok(0);
        }
        let Ok(syndrome) = GenericGFPoly::new(self.field, &syndrome_coefficients) else {
            return Err(Exceptions::REED_SOLOMON);
        };
        let monomial = GenericGF::build_monomial(self.field, two_s as usize, 1)?;
        let sigma_omega =
            self.run_euclidean_algorithm(&monomial, &syndrome, two_s as usize)?;
        let sigma = &sigma_omega[0];
        let omega = &sigma_omega[1];
        let error_locations = self.find_error_locations(sigma)?;
        let error_magnitudes = self.find_error_magnitudes(omega, &error_locations)?;
        for (error_location, error_magnitude) in error_locations.iter().zip(error_magnitudes) {
            let log_value = self.field.log(*error_location as i32)?;
            if log_value > received.len() as i32 - 1 {
                return Err(Exceptions::reed_solomon_with("Bad error location"));
            }
            let position: isize = received.len() as isize - 1 - log_value as isize;
            if position < 0 {
                return Err(Exceptions::reed_solomon_with("Bad error location"));
            }
            received[position as usize] =
                GenericGF::add_or_subtract(received[position as usize], error_magnitude);
        }
        Ok(error_locations.len())
    }

    fn run_euclidean_algorithm(
        &self,
        a: &GenericGFPoly,
        b: &GenericGFPoly,
        degree_limit: usize,
    ) -> Result<Vec<GenericGFPoly>> {
        // Assume a's degree is >= b's
        let mut a = a.clone();
        let mut b = b.clone();
        if a.get_degree() < b.get_degree() {
            std::mem::swap(&mut a, &mut b);
        }

        let mut r_last = a;
        let mut r = b;
        let mut t_last = r_last.get_zero()?;
        let mut t = r_last.get_one()?;

        // Run Euclidean algorithm until r's degree is less than r/2
        while 2 * r.get_degree() >= degree_limit {
            let r_last_last = r_last;
            let t_last_last = t_last;
            r_last = r;
            t_last = t;

            // Divide r_last_last by r_last, with quotient in q and remainder in r
            if r_last.is_zero() {
                // Oops, Euclidean algorithm already terminated?
                return Err(Exceptions::reed_solomon_with("r_{i-1} was zero"));
            }
            r = r_last_last;
            let mut q = r.get_zero()?;
            let denominator_leading_term = r_last.get_coefficient(r_last.get_degree());
            let dlt_inverse = self.field.inverse(denominator_leading_term)?;
            while r.get_degree() >= r_last.get_degree() && !r.is_zero() {
                let degree_diff = r.get_degree() - r_last.get_degree();
                let scale = self
                    .field
                    .multiply(r.get_coefficient(r.get_degree()), dlt_inverse);
                let monomial = GenericGF::build_monomial(self.field, degree_diff, scale)?;
                q = q.add_or_subtract(&monomial)?;
                r = r.add_or_subtract(&r_last.multiply_by_monomial(degree_diff, scale)?)?;
            }

            t = (q.multiply(&t_last)?).add_or_subtract(&t_last_last)?;

            if r.get_degree() >= r_last.get_degree() {
                return Err(Exceptions::reed_solomon_with(format!(
                    "Division algorithm failed to reduce polynomial? r: {r}, r_last: {r_last}"
                )));
            }
        }

        let sigma_tilde_at_zero = t.get_coefficient(0);
        if sigma_tilde_at_zero == 0 {
            return Err(Exceptions::reed_solomon_with("sigmaTilde(0) was zero"));
        }

        let Ok(inverse) = self.field.inverse(sigma_tilde_at_zero) else {
            return Err(Exceptions::reed_solomon_with("ArithmetricException"));
        };
        let sigma = t.multiply_with_scalar(inverse)?;
        let omega = r.multiply_with_scalar(inverse)?;
        Ok(vec![sigma, omega])
    }

    fn find_error_locations(&self, error_locator: &GenericGFPoly) -> Result<Vec<usize>> {
        // This is a direct application of Chien's search
        let num_errors = error_locator.get_degree();
        if num_errors == 1 {
            // shortcut
            return Ok(vec![error_locator.get_coefficient(1) as usize]);
        }

        let mut result: Vec<usize> = vec![0; num_errors];
        let mut e = 0;
        for i in 1..self.field.get_size() {
            if e >= num_errors {
                break;
            }
            if error_locator.evaluate_at(i) == 0 {
                result[e] = self.field.inverse(i as i32)? as usize;
                e += 1;
            }
        }
        if e != num_errors {
            return Err(Exceptions::reed_solomon_with(
                "Error locator degree does not match number of roots",
            ));
        }
        Ok(result)
    }

    fn find_error_magnitudes(
        &self,
        error_evaluator: &GenericGFPoly,
        error_locations: &[usize],
    ) -> Result<Vec<i32>> {
        // This is directly applying Forney's Formula
        let s = error_locations.len();
        let mut result = vec![0; s];
        for i in 0..s {
            let xi_inverse = self.field.inverse(error_locations[i] as i32)?;
            let mut denominator = 1;
            for (j, loc) in error_locations.iter().enumerate().take(s) {
                if i != j {
                    // The naive multiplication fails on some Apple and Linux JDKs due to a Hotspot bug.
                    // Below is a funny-looking workaround from Steven Parkes.
                    let term = self.field.multiply(*loc as i32, xi_inverse);
                    let term_plus1 = if (term & 0x1) == 0 {
                        term | 1
                    } else {
                        term & !1
                    };
                    denominator = self.field.multiply(denominator, term_plus1);
                }
            }
            result[i] = self.field.multiply(
                error_evaluator.evaluate_at(xi_inverse as usize),
                self.field.inverse(denominator)?,
            );
            if self.field.get_generator_base() != 0 {
                result[i] = self.field.multiply(result[i], xi_inverse);
            }
        }
        Ok(result)
    }
}
