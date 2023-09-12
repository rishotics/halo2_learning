use std::{marker::PhantomData, f32::consts::E};

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::*,
    poly::Rotation,
    pasta::Fp, dev::MockProver
};
// #[derive(Debug, Clone)] is a Rust attribute used to automatically generate implementations of the Debug and Clone traits for a struct
#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F,F>);

#[derive(Debug, Clone)]
struct FiboConfig{
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
}

#[derive(Debug, Clone)]
struct FiboChip<F: FieldExt> {
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    fn construct(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) ->FiboConfig {
        let col_a: Column<Advice> = meta.advice_column();
        let col_b: Column<Advice> = meta.advice_column();
        let col_c: Column<Advice> = meta.advice_column();

        let selector: Selector = meta.selector();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);

        meta.create_gate("add", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());

            vec![s* (a+b-c)]
        });

        FiboConfig {
            advice: [col_a, col_b, col_c],
            selector
        }
    }

    fn assign_first_row(&self, mut layouter: impl Layouter<F>, a: Option<F>, b: Option<F>) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "first row", |mut region|{
            self.config.selector.enable(&mut region, 0);

            let a_cell = region.assign_advice(
            || "a", 
            self.config.advice[0], 
            0,
             || a.ok_or(Error::Synthesis),
            ).map(ACell)?;

            let b_cell = region.assign_advice(
            || "b", 
            self.config.advice[1], 
            0,
             || b.ok_or(Error::Synthesis),
            ).map(ACell)?;


            let c_val = a.and_then(|a| b.map(|b| a+b));

            let c_cell = region.assign_advice(
                || "c", 
                self.config.advice[2], 
                0, 
            || c_val.ok_or(Error::Synthesis),
            ).map(ACell)?;

            Ok((a_cell, b_cell, c_cell))

        })
    }


    fn assign_row(&self, mut layouter: impl Layouter<F>, prev_b: &ACell<F>, prev_c: &ACell<F>) -> Result<ACell<F>,Error> {

        layouter.assign_region(
            || "next row", 
        |mut region| {
            self.config.selector.enable(&mut region, 0);
            prev_b.0.copy_advice(|| "a", &mut region, self.config.advice[0], 0)?;
            prev_c.0.copy_advice(|| "b", &mut region, self.config.advice[1], 0)?;

            let c_val = prev_b.0.value().and_then(
                |b| {
                    prev_c.0.value().map(|c| *b + *c)
                }
            );

            let c_cell = region.assign_advice(
                || "c", 
                self.config.advice[2], 
                0,
                || c_val.ok_or(Error::Synthesis),
            ).map(ACell)?;

            Ok((c_cell))

        })
    }

}


#[derive(Default)]
struct MyCircuit<F> {
    pub a:Option<F>,
    pub b:Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    // It generates an empty circuit without any witness
    // You can use this api to generate proving key or verification key without any witness
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    // create configuration for the Circuit
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FiboChip::configure(meta)
    } 
    
    // API to be called after the constraint system is defined.
    // Assign the values inside the actual prover input inside the circuit.
    // mut layouter: impl Layouter<F> specifies a function parameter named layouter, which is mutable (mut keyword), and implements the Layouter<F> trait.
    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        // We create a new instance of chip using the config passed as input
        let chip = FiboChip::construct(config);
        // now we assign stuff inside the circuit!
        // first row is particular so we create a specific function for that.
        // This function will take as input the "a" and "b" value passed to instantiate the circuit
        // We also use a layouter as this is a good way to separate different regions of the circuit
        // We can also assign name to the layouter
        let (_, mut prev_b, mut prev_c) = chip.assign_first_row(layouter.namespace(|| "first row"), self.a, self.b)?;

        // Now we have assigned the first row! Now we have to assign the other rows! Remember that the idea of the circuit was
        // // given f(0) = x, f(1) = y, we will prove f(9) = z. We already have assigned f(0) and f(1). We now need to assign values to the other rows. 
        for _i in 3..10 {
            let c_cell  = chip.assign_row(
                layouter.namespace(|| "next row"),
                &prev_b,
                &prev_c,
            )?;

            prev_b = prev_c;
            prev_c = c_cell;
        }

        Ok(())
    }

}



fn main() { 
    let k = 4;
    let a = Fp::from(1);
    let b = Fp::from(1);

    let circuit = MyCircuit {
        a: Some(a),
        b: Some(b),
    };

    // The mock prover is a function that execute the configuration of the circuit by running its method configure
    // and also execute the syntetize function, by passing in the actual input.
    // The instance vector is empty as we don't have any public input to pass to the function
    let prover = MockProver::run(k, &circuit, vec![]).unwrap();

    prover.assert_satisfied();

}


