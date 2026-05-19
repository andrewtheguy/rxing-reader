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

use std::fmt;

use crate::Exceptions;
use crate::common::Result;

use super::{GenericGF, GenericGFRef};

/**
 * <p>Represents a polynomial whose coefficients are elements of a GF.
 * Instances of this class are immutable.</p>
 *
 * <p>Much credit is due to William Rucklidge since portions of this code are an indirect
 * port of his C++ Reed-Solomon implementation.</p>
 *
 * @author Sean Owen
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericGFPoly {
    field: GenericGFRef,
    coefficients: Vec<i32>,
}

impl GenericGFPoly {
    /**
     * @param field the {@link GenericGF} instance representing the field to use
     * to perform computations
     * @param coefficients coefficients as ints representing elements of GF(size), arranged
     * from most significant (highest-power term) coefficient to least significant
     * @throws IllegalArgumentException if argument is null or empty,
     * or if leading coefficient is 0 and this is not a
     * constant polynomial (that is, it is not the monomial "0")
     */
    pub fn new(field: GenericGFRef, coefficients: &[i32]) -> Result<Self> {
        if coefficients.is_empty() {
            return Err(Exceptions::illegal_argument_with(
                "coefficients cannot be empty",
            ));
        }
        Ok(Self {
            field,
            coefficients: {
                let coefficients_length = coefficients.len();
                if coefficients_length > 1 && coefficients[0] == 0 {
                    // Leading term must be non-zero for anything except the constant polynomial "0"
                    let mut first_non_zero = 1;
                    while first_non_zero < coefficients_length && coefficients[first_non_zero] == 0
                    {
                        first_non_zero += 1;
                    }
                    if first_non_zero == coefficients_length {
                        vec![0]
                    } else {
                        let mut new_coefficients = vec![0; coefficients_length - first_non_zero];
                        let l = new_coefficients.len() - 1;
                        new_coefficients[0..=l].clone_from_slice(&coefficients[first_non_zero..]);
                        new_coefficients
                    }
                } else {
                    coefficients.to_vec()
                }
            },
        })
    }

    pub fn get_coefficients(&self) -> &Vec<i32> {
        &self.coefficients
    }

    /**
     * @return degree of this polynomial
     */
    pub fn get_degree(&self) -> usize {
        self.coefficients.len() - 1
    }

    /**
     * @return true iff this polynomial is the monomial "0"
     */
    pub fn is_zero(&self) -> bool {
        self.coefficients[0] == 0
    }

    /**
     * @return coefficient of x^degree term in this polynomial
     */
    pub fn get_coefficient(&self, degree: usize) -> i32 {
        self.coefficients[self.coefficients.len() - 1 - degree]
    }

    /**
     * @return evaluation of this polynomial at a given point
     */
    pub fn evaluate_at(&self, a: usize) -> i32 {
        if a == 0 {
            // Just return the x^0 coefficient
            return self.get_coefficient(0);
        }
        if a == 1 {
            // Just the sum of the coefficients
            let mut result = 0;
            for coefficient in &self.coefficients {
                result = GenericGF::add_or_subtract(result, *coefficient);
            }
            return result;
        }
        let mut result = self.coefficients[0];
        let size = self.coefficients.len();
        for i in 1..size {
            result = GenericGF::add_or_subtract(
                self.field.multiply(a as i32, result),
                self.coefficients[i],
            );
        }
        result
    }

    pub fn add_or_subtract(&self, other: &GenericGFPoly) -> Result<GenericGFPoly> {
        if self.field != other.field {
            return Err(Exceptions::illegal_argument_with(
                "GenericGFPolys do not have same GenericGF field",
            ));
        }
        if self.is_zero() {
            return Ok(other.clone());
        }
        if other.is_zero() {
            return Ok(self.clone());
        }

        let mut smaller_coefficients = self.coefficients.clone();
        let mut larger_coefficients = other.coefficients.clone();
        if smaller_coefficients.len() > larger_coefficients.len() {
            std::mem::swap(&mut smaller_coefficients, &mut larger_coefficients)
        }

        let mut sum_diff = vec![0; larger_coefficients.len()];
        let length_diff = larger_coefficients.len() - smaller_coefficients.len();
        // Copy high-order terms only found in higher-degree polynomial's coefficients
        sum_diff[0..length_diff].clone_from_slice(&larger_coefficients[0..length_diff]);

        for i in length_diff..larger_coefficients.len() {
            sum_diff[i] = GenericGF::add_or_subtract(
                smaller_coefficients[i - length_diff],
                larger_coefficients[i],
            );
        }

        GenericGFPoly::new(self.field, &sum_diff)
    }

    pub fn multiply(&self, other: &GenericGFPoly) -> Result<GenericGFPoly> {
        if self.field != other.field {
            return Err(Exceptions::illegal_argument_with(
                "GenericGFPolys do not have same GenericGF field",
            ));
        }
        if self.is_zero() || other.is_zero() {
            return Ok(self.get_zero());
        }
        let a_coefficients = self.coefficients.clone();
        let a_length = a_coefficients.len();
        let b_coefficients = other.coefficients.clone();
        let b_length = b_coefficients.len();
        let mut product = vec![0; a_length + b_length - 1];
        for i in 0..a_length {
            let a_coeff = a_coefficients[i];
            for j in 0..b_length {
                product[i + j] = GenericGF::add_or_subtract(
                    product[i + j],
                    self.field.multiply(a_coeff, b_coefficients[j]),
                );
            }
        }
        GenericGFPoly::new(self.field, &product)
    }

    pub fn multiply_with_scalar(&self, scalar: i32) -> GenericGFPoly {
        if scalar == 0 {
            return self.get_zero();
        }
        if scalar == 1 {
            return self.clone();
        }
        let size = self.coefficients.len();

        let mut product = vec![0; size];
        for (i, prod) in product.iter_mut().enumerate().take(size) {
            *prod = self.field.multiply(self.coefficients[i], scalar);
        }
        GenericGFPoly::new(self.field, &product).unwrap()
    }

    pub fn get_zero(&self) -> Self {
        GenericGFPoly::new(self.field, &[0]).unwrap()
    }

    pub fn get_one(&self) -> Self {
        GenericGFPoly::new(self.field, &[1]).unwrap()
    }

    pub fn multiply_by_monomial(&self, degree: usize, coefficient: i32) -> Result<GenericGFPoly> {
        if coefficient == 0 {
            return Ok(self.get_zero());
        }
        let size = self.coefficients.len();
        let mut product = vec![0; size + degree];
        for (i, prod) in product.iter_mut().enumerate().take(size) {
            *prod = self.field.multiply(self.coefficients[i], coefficient);
        }
        GenericGFPoly::new(self.field, &product)
    }

    pub fn divide(&self, other: &GenericGFPoly) -> Result<(GenericGFPoly, GenericGFPoly)> {
        if self.field != other.field {
            return Err(Exceptions::illegal_argument_with(
                "GenericGFPolys do not have same GenericGF field",
            ));
        }
        if other.is_zero() {
            return Err(Exceptions::illegal_argument_with("Divide by 0"));
        }

        let mut quotient = self.get_zero();
        let mut remainder = self.clone();

        let denominator_leading_term = other.get_coefficient(other.get_degree());
        let inverse_denominator_leading_term = match self.field.inverse(denominator_leading_term) {
            Ok(val) => val,
            Err(_issue) => return Err(Exceptions::illegal_argument_with("arithmetic issue")),
        };

        while remainder.get_degree() >= other.get_degree() && !remainder.is_zero() {
            let degree_difference = remainder.get_degree() - other.get_degree();
            let scale = self.field.multiply(
                remainder.get_coefficient(remainder.get_degree()),
                inverse_denominator_leading_term,
            );
            let term = other.multiply_by_monomial(degree_difference, scale)?;
            let iteration_quotient = GenericGF::build_monomial(self.field, degree_difference, scale);
            quotient = quotient.add_or_subtract(&iteration_quotient)?;
            remainder = remainder.add_or_subtract(&term)?;
        }

        Ok((quotient, remainder))
    }
}

impl fmt::Display for GenericGFPoly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }
        let mut result = String::with_capacity(8 * self.get_degree());
        for degree in (0..=self.get_degree()).rev() {
            let mut coefficient = self.get_coefficient(degree);
            if coefficient != 0 {
                if coefficient < 0 {
                    if degree == self.get_degree() {
                        result.push('-');
                    } else {
                        result.push_str(" - ");
                    }
                    coefficient = -coefficient;
                } else if !result.is_empty() {
                    result.push_str(" + ");
                }
                if (degree == 0 || coefficient != 1)
                    && let Ok(alpha_power) = self.field.log(coefficient)
                {
                    if alpha_power == 0 {
                        result.push('1');
                    } else if alpha_power == 1 {
                        result.push('a');
                    } else {
                        result.push_str("a^");
                        result.push_str(&format!("{alpha_power}"));
                    }
                }
                if degree != 0 {
                    if degree == 1 {
                        result.push('x');
                    } else {
                        result.push_str("x^");
                        result.push_str(&format!("{degree}"));
                    }
                }
            }
        }
        write!(f, "{result}")
    }
}
