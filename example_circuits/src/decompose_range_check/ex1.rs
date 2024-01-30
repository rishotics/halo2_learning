

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{
        Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector,
        TableColumn,
    },
    poly::Rotation,
};



pub struct DecomposeConfig<F: FieldExt, const LOOKUP_RANGE: usize> {
    running_sum: Column<Advice>,
    q_decompose: Selector,
    table: RangeTableConfig<F, LOOKUP_RANGE>,
    _marker: std::marker::PhantomData<F>,
}

impl<F: FieldExt, const LOOKUP_RANGE: usize> DecomposeConfig<F, LOOKUP_RANGE> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
    ) -> Self {
        let running_sum = meta.advice_column();
        let q_decompose = meta.complex_selector();

        let table = RangeTableConfig::configure(meta);
        
        meta.lookup(|meta| {
            let q_decompose = meta.query_selector(q_decompose);
            let z_curr = meta.query_advice(running_sum, Rotation::cur());
            let z_next = meta.query_advice(running_sum, Rotation::next());

            //we need to fix a column for constraint constant step used to enforce z_C == 0;
            let constant = meta.fixed_column();
            meta.enable_contant(constant);
            //similarily we need to enable 'running sum' to participate in the parmutation 
            meta.enable_equality(running_sum);

            let lookup_num_bits = 
                log2_ceil(LOOKUP_RANGE as u64);
            let chunk = z_curr - z_next * Expression::Constant(F::from_u64(1u64<< lookup_num_bits));

            let not_q_decompose = Expression::Constant(F::one()) - q_decompose.clone();
            let default_chunk = Expression::Constant(F::zero());

            let expr = not_q_decompose * default_chunk + q_decompose * chunk;

            vec![
                (q_decompose * chunk, table.value) 
            ]
        })

        Self {
            running_sum,
            q_decompose,
            table,
            _marker: std::marker::PhantomData,
        }
        
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        // this is assigned cell not normal value, this means this value is used before
        value: AssignedCell<Assigned<F>, F>,
        num_bits: usize,
    ) -> Result< (), Error> {
        layouter.assign_region(|| "Decompose value", |mut region| {
            let mut offset = 0;

            // 0. copy in the witness value
            let mut z= value.copy_advice(|| "copy value to init running sum", 
                &mut region, 
                self.running_sum, 
                offset)?;

            //1 compute the interstitial running sum values(z_1, z_2, ..., z_C)
            // transpose: ->  Value<Vec<Assigned<F>> -> Vec<Value<Assigned<F>>
            let lookup_num_bits = log2_ceil(LOOKUP_RANGE as u64 );
            let running_sum = value.value().map(|&v| compute_running_sum(v, num_bits, lookup_num_bits)).transpose_vec(num_bits/lookup_num_bits);

            //2 assign the running sum values
            for z_i in running_sum.into_iter() {
                z = region.assign_advice(|| format!("assign z_{}", offset), self.running_sum, offset, || z_i)?;
                offset += 1;    
            }

            //3. enable selector on each row of the running sum
            for row in (0..(num_bits/lookup_num_bits)) {
                self.q_decompose.enable(&mut region, row)?;
            }

            //4. constrain the final rumnning sum 'z_c' == 0
            ///constrain constant: assume that the circuit has a fixed column available where we can witness `constant`.
            /// Returns an error if the cell is in a column where equality has not been enabled.
            /// 
            region.constrain_contstant(z_i.cell(), F::zero());





        })
    }


}


fn compute_running_sum<F: FieldExt + PrimeFieldBits, const LOOKUP_NUM_BITS: usize> (
    value: Assigned<F>,
    num_bits: usize,
) -> Vec<Assigned<F>> {

}

#[test]

fn test_here(){
    println!("Hello, world!");
}

fn lebs2ip(bits: &[bool]) -> u64 {
    assert!(bits.len() <= 64);
    bits.iter()
        .enumerate()
        .fold(0u64, |acc, (i, b)| acc + if *b { 1 << i } else { 0 })
}

// Function to compute the interstitial running sum values {z_1, ..., z_C}}
fn compute_running_sum<F: FieldExt + PrimeFieldBits>(
    value: Assigned<F>,
    num_bits: usize,
    lookup_num_bits: usize,
) -> Vec<Assigned<F>> {
    let mut running_sum = vec![];
    let mut z = value;

    // Get the little-endian bit representation of `value`.
    let value: Vec<_> = value
        .evaluate()
        .to_le_bits()
        .iter()
        .by_vals()
        .take(num_bits)
        .collect();
    for chunk in value.chunks(LOOKUP_NUM_BITS) {
        let chunk = Assigned::from(F::from(lebs2ip(chunk)));
        // z_{i+1} = (z_i - c_i) * 2^{-K}:
        z = (z - chunk) * Assigned::from(F::from(1u64 << LOOKUP_NUM_BITS)).invert();
        running_sum.push(z);
    }

    assert_eq!(running_sum.len(), num_bits / LOOKUP_NUM_BITS);
    running_sum
}