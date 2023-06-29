use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::*,
};


#[derive(Debug, Clone)]

struct FiboConfig {
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
}

struct FiboChip<F: FieldExt>{
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    fn construct(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }


    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FiboConfig {
        let col_a : Column<Advice> = meta.advice_column();
        let col_b : Column<Advice> = meta.advice_column();
        let col_c : Column<Advice> = meta.advice_column();
        let selector: 
    }

    fn assign(){

    }

    
}

fn main() {
    println!("Hello, world!");
}
