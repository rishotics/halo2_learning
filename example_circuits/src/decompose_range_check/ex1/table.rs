use std::{marker::PhantomData, os::unix::raw::off_t};

use halo2_proofs::{plonk::{Error, TableColumn, ConstraintSystem}, arithmetic::FieldExt, circuit::{Layouter, Value}};

/// A lookup table of values of NUM_BITS length.
/// e.g NUM_BITS=8 values=[0..255]; RANGE = 2^NUM_BITS
/// 
#[derive(Debug, Clone)]
pub(super) struct RangeCheckTable<F: FieldExt, const RANGE: usize> {
    pub(super) value: TableColumn,
    _marker: PhantomData<F>,
}

 impl<F: FieldExt, const RANGE: usize> RangeCheckTable<F, RANGE> {


    pub(super) fn configure(
        meta: &mut ConstraintSystem<F>,
    ) -> Self {
        let value = meta.lookup_table_column();
        Self {value, _marker: PhantomData}
    }

    //load functioon loads all the fixed values into the table 
    //and this is done at the key gen time
    pub(super) fn load(
        &self,
        layouter: &mut impl Layouter<F>,
    ) -> Result<(), Error> {
        
        //a special API for lookup table
        layouter.assign_table(|| "load range-check table", |mut table| {
            let mut offset = 0;
            //for some NUM BITS we want to load all the values into the row
            for i in 0..(RANGE) {
                table.assign_cell(|| "assign cell", self.value, offset, || Value::known(F::from((i*i) as u64)))?;
                offset += 1;
            }
            Ok(())
        })
    }
 }