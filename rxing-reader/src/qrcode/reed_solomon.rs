#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReedSolomonError {
    TooManyErrors,
}

pub(super) fn correct_qr_errors(
    received: &mut [u8],
    num_data_codewords: usize,
) -> Result<(), ReedSolomonError> {
    let two_s = received
        .len()
        .checked_sub(num_data_codewords)
        .ok_or(ReedSolomonError::TooManyErrors)?;
    if two_s == 0 {
        return Ok(());
    }
    if received.len() >= FIELD_SIZE {
        return Err(ReedSolomonError::TooManyErrors);
    }

    let field = &QR_FIELD;
    let mut syndrome_coefficients = vec![0; two_s];
    let mut no_error = true;
    for i in 0..two_s {
        let eval = poly_eval(field, received, field.exp(i));
        syndrome_coefficients[two_s - 1 - i] = eval;
        no_error &= eval == 0;
    }
    if no_error {
        return Ok(());
    }

    let syndrome = poly_new(syndrome_coefficients);
    let (sigma, omega) =
        run_euclidean_algorithm(field, build_monomial(two_s, 1), syndrome, two_s)?;
    let error_locations = find_error_locations(field, &sigma)?;
    let error_magnitudes = find_error_magnitudes(field, &omega, &error_locations)?;

    for (&location, &magnitude) in error_locations.iter().zip(error_magnitudes.iter()) {
        let log = field.log(location)?;
        if log >= received.len() {
            return Err(ReedSolomonError::TooManyErrors);
        }
        let position = received.len() - 1 - log;
        received[position] ^= magnitude;
    }

    if has_errors(field, received, two_s) {
        return Err(ReedSolomonError::TooManyErrors);
    }

    Ok(())
}

const FIELD_SIZE: usize = 256;
const FIELD_ORDER: usize = FIELD_SIZE - 1;
const PRIMITIVE: usize = 0x011D;

static QR_FIELD: QrField = QrField::new();

struct QrField {
    exp_table: [u8; FIELD_SIZE],
    log_table: [usize; FIELD_SIZE],
}

impl QrField {
    const fn new() -> Self {
        let mut exp_table = [0; FIELD_SIZE];
        let mut log_table = [0; FIELD_SIZE];
        let mut x = 1;
        let mut i = 0;
        while i < FIELD_SIZE {
            exp_table[i] = x as u8;
            x <<= 1;
            if x >= FIELD_SIZE {
                x ^= PRIMITIVE;
                x &= FIELD_SIZE - 1;
            }
            i += 1;
        }
        i = 0;
        while i < FIELD_ORDER {
            let value = exp_table[i] as usize;
            log_table[value] = i;
            i += 1;
        }

        Self {
            exp_table,
            log_table,
        }
    }

    fn exp(&self, value: usize) -> u8 {
        self.exp_table[value]
    }

    fn log(&self, value: u8) -> Result<usize, ReedSolomonError> {
        if value == 0 {
            return Err(ReedSolomonError::TooManyErrors);
        }
        Ok(self.log_table[usize::from(value)])
    }

    fn inverse(&self, value: u8) -> Result<u8, ReedSolomonError> {
        if value == 0 {
            return Err(ReedSolomonError::TooManyErrors);
        }
        Ok(self.exp_table[FIELD_ORDER - self.log_table[usize::from(value)]])
    }

    fn multiply(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        let log_sum = self.log_table[usize::from(a)] + self.log_table[usize::from(b)];
        self.exp_table[log_sum % FIELD_ORDER]
    }
}

fn has_errors(field: &QrField, received: &[u8], two_s: usize) -> bool {
    (0..two_s).any(|i| poly_eval(field, received, field.exp(i)) != 0)
}

fn run_euclidean_algorithm(
    field: &QrField,
    mut a: Vec<u8>,
    mut b: Vec<u8>,
    received_ec_len: usize,
) -> Result<(Vec<u8>, Vec<u8>), ReedSolomonError> {
    if poly_degree(&a) < poly_degree(&b) {
        std::mem::swap(&mut a, &mut b);
    }

    let mut r_last = a;
    let mut r = b;
    let mut t_last = vec![0];
    let mut t = vec![1];

    while poly_degree(&r) >= received_ec_len / 2 {
        let r_last_last = r_last;
        let t_last_last = t_last;
        r_last = r;
        t_last = t;

        if poly_is_zero(&r_last) {
            return Err(ReedSolomonError::TooManyErrors);
        }

        r = r_last_last;
        let quotient_degree = poly_degree(&r) - poly_degree(&r_last);
        let mut q = vec![0; quotient_degree + 1];
        let denominator_leading_term = poly_coefficient(&r_last, poly_degree(&r_last));
        let dlt_inverse = field.inverse(denominator_leading_term)?;

        while !poly_is_zero(&r) && poly_degree(&r) >= poly_degree(&r_last) {
            let degree_diff = poly_degree(&r) - poly_degree(&r_last);
            let scale = field.multiply(poly_coefficient(&r, poly_degree(&r)), dlt_inverse);
            q[quotient_degree - degree_diff] ^= scale;
            poly_add_scaled_in_place(field, &mut r, &r_last, scale);
        }
        poly_trim(&mut q);

        t = poly_add(poly_multiply(field, &q, &t_last), &t_last_last);
        if poly_degree(&r) >= poly_degree(&r_last) {
            return Err(ReedSolomonError::TooManyErrors);
        }
    }

    let sigma_tilde_at_zero = poly_coefficient(&t, 0);
    if sigma_tilde_at_zero == 0 {
        return Err(ReedSolomonError::TooManyErrors);
    }

    let inverse = field.inverse(sigma_tilde_at_zero)?;
    Ok((
        poly_multiply_scalar(field, t, inverse),
        poly_multiply_scalar(field, r, inverse),
    ))
}

fn find_error_locations(
    field: &QrField,
    error_locator: &[u8],
) -> Result<Vec<u8>, ReedSolomonError> {
    let num_errors = poly_degree(error_locator);
    if num_errors == 1 {
        return Ok(vec![poly_coefficient(error_locator, 1)]);
    }

    let mut result = Vec::with_capacity(num_errors);
    for value in 1..=u8::MAX {
        if poly_eval(field, error_locator, value) == 0 {
            result.push(field.inverse(value)?);
            if result.len() == num_errors {
                break;
            }
        }
    }

    if result.len() == num_errors {
        Ok(result)
    } else {
        Err(ReedSolomonError::TooManyErrors)
    }
}

fn find_error_magnitudes(
    field: &QrField,
    error_evaluator: &[u8],
    error_locations: &[u8],
) -> Result<Vec<u8>, ReedSolomonError> {
    let mut result = Vec::with_capacity(error_locations.len());
    for (i, &location) in error_locations.iter().enumerate() {
        let xi_inverse = field.inverse(location)?;
        let mut denominator = 1;

        for (j, &other_location) in error_locations.iter().enumerate() {
            if i != j {
                let term = field.multiply(other_location, xi_inverse) ^ 1;
                denominator = field.multiply(denominator, term);
            }
        }

        let magnitude = field.multiply(
            poly_eval(field, error_evaluator, xi_inverse),
            field.inverse(denominator)?,
        );
        result.push(magnitude);
    }
    Ok(result)
}

fn build_monomial(degree: usize, coefficient: u8) -> Vec<u8> {
    if coefficient == 0 {
        return vec![0];
    }
    let mut coefficients = vec![0; degree + 1];
    coefficients[0] = coefficient;
    coefficients
}

fn poly_new(mut coefficients: Vec<u8>) -> Vec<u8> {
    if coefficients.is_empty() {
        return vec![0];
    }
    poly_trim(&mut coefficients);
    coefficients
}

fn poly_trim(poly: &mut Vec<u8>) {
    match poly.iter().position(|&coefficient| coefficient != 0) {
        Some(0) => {}
        Some(first_non_zero) => {
            poly.drain(..first_non_zero);
        }
        None => {
            poly.clear();
            poly.push(0);
        }
    }
}

fn poly_is_zero(poly: &[u8]) -> bool {
    poly[0] == 0
}

fn poly_degree(poly: &[u8]) -> usize {
    poly.len() - 1
}

fn poly_coefficient(poly: &[u8], degree: usize) -> u8 {
    poly[poly.len() - 1 - degree]
}

fn poly_eval(field: &QrField, poly: &[u8], value: u8) -> u8 {
    if value == 0 {
        return poly_coefficient(poly, 0);
    }
    if value == 1 {
        return poly.iter().fold(0, |accumulator, &coefficient| {
            accumulator ^ coefficient
        });
    }

    let mut result = poly[0];
    for &coefficient in poly.iter().skip(1) {
        result = field.multiply(result, value) ^ coefficient;
    }
    result
}

fn poly_add(a: Vec<u8>, b: &[u8]) -> Vec<u8> {
    if poly_is_zero(&a) {
        return b.to_vec();
    }
    if poly_is_zero(b) {
        return a;
    }

    let (mut sum, smaller) = if a.len() >= b.len() {
        (a, b)
    } else {
        (b.to_vec(), a.as_slice())
    };
    let length_diff = sum.len() - smaller.len();
    for (target, &coefficient) in sum[length_diff..].iter_mut().zip(smaller.iter()) {
        *target ^= coefficient;
    }
    poly_new(sum)
}

fn poly_multiply(field: &QrField, a: &[u8], b: &[u8]) -> Vec<u8> {
    if poly_is_zero(a) || poly_is_zero(b) {
        return vec![0];
    }

    let mut product = vec![0; a.len() + b.len() - 1];
    for (i, &a_coefficient) in a.iter().enumerate() {
        if a_coefficient == 0 {
            continue;
        }
        for (j, &b_coefficient) in b.iter().enumerate() {
            if b_coefficient != 0 {
                product[i + j] ^= field.multiply(a_coefficient, b_coefficient);
            }
        }
    }
    poly_new(product)
}

fn poly_multiply_scalar(field: &QrField, mut poly: Vec<u8>, scalar: u8) -> Vec<u8> {
    if scalar == 0 || poly_is_zero(&poly) {
        return vec![0];
    }
    if scalar == 1 {
        return poly;
    }

    for coefficient in &mut poly {
        *coefficient = field.multiply(*coefficient, scalar);
    }
    poly
}

fn poly_add_scaled_in_place(field: &QrField, target: &mut Vec<u8>, term: &[u8], scale: u8) {
    if scale == 0 || poly_is_zero(term) {
        return;
    }
    debug_assert!(target.len() >= term.len());

    // Multiplication by x^degree_diff appends low-degree zero terms, so the
    // scaled divisor aligns at the leading coefficient.
    for (target_coefficient, &term_coefficient) in target.iter_mut().zip(term.iter()) {
        if term_coefficient != 0 {
            *target_coefficient ^= field.multiply(term_coefficient, scale);
        }
    }
    poly_trim(target);
}

#[cfg(test)]
mod tests {
    use super::correct_qr_errors;

    const DATA_LEN: usize = 11;
    const ENCODED: [u8; 20] = [
        104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 145, 124, 96, 105, 94, 31, 179, 149,
        163,
    ];

    #[test]
    fn leaves_clean_codewords_unchanged() {
        let mut codewords = ENCODED;
        correct_qr_errors(&mut codewords, DATA_LEN).unwrap();
        assert_eq!(codewords, ENCODED);
    }

    #[test]
    fn corrects_data_codeword_errors() {
        let mut codewords = ENCODED;
        codewords[0] = 0;
        codewords[3] = 2;
        codewords[10] = 88;

        correct_qr_errors(&mut codewords, DATA_LEN).unwrap();

        assert_eq!(&codewords[..DATA_LEN], b"hello world");
    }

    #[test]
    fn corrects_every_single_codeword_error() {
        for position in 0..ENCODED.len() {
            for delta in 1..=u8::MAX {
                let mut codewords = ENCODED;
                codewords[position] ^= delta;

                correct_qr_errors(&mut codewords, DATA_LEN).unwrap_or_else(|_| {
                    panic!("failed to correct position={position}, delta=0x{delta:02X}")
                });

                assert_eq!(
                    codewords, ENCODED,
                    "wrong correction for position={position}, delta=0x{delta:02X}"
                );
            }
        }
    }

    #[test]
    fn corrects_max_capacity_mixed_errors() {
        let mut codewords = ENCODED;
        for (position, delta) in [(0, 0x55), (5, 0xE3), (11, 0x42), (19, 0x99)] {
            codewords[position] ^= delta;
        }

        correct_qr_errors(&mut codewords, DATA_LEN).unwrap();

        assert_eq!(codewords, ENCODED);
    }

    #[test]
    fn rejects_too_many_errors() {
        let mut codewords = ENCODED;
        for codeword in codewords.iter_mut().take(6) {
            *codeword ^= 0xFF;
        }

        assert!(correct_qr_errors(&mut codewords, DATA_LEN).is_err());
    }
}
