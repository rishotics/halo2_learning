use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE:usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    _marker: PhantomData<F>,

}


impl<F: FieldExt, const RANGE: usize> RangeCheckConfig<F, RANGE> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>,
    ) -> Self {
        let q_range_check = meta.selector();

        let config = Self {
            q_range_check,
            value,
            _marker: PhantomData
        };

        //Range check gate
        meta.create_gate("Range check", |meta| {
            //when we query selectors we dont query rotations as the default value
            // is at the current row. query_selector gives an expression on the selector
            let q_range_check = meta.query_selector(q_range_check);

            //get the value at the current row 
            let value = meta.query_advice(value, Rotation::cur());

            //vector of expressions at the end of create gate
            let range_check = |range: usize, value: Expression<F>| {
                (0..range).fold(value.clone(), |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64)) -value.clone())
                })
            };

            //you select one selector and with-selector multiplies each expression with 
            // the selector. so basically abstracts away the selector.
            Constraints::with_selector(q_range_check, [("range_check", range_check(RANGE, value))])
        });

        config
    }

    //a lot of overhead in remembering the layout of the template
    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
    ) -> Result<(), Error> {
        layouter.assign_region(|| "Assign value", |mut region| {
            let offset = 0;

            //enable q range check. what is region?
            self.q_range_check.enable(&mut region, offset);

            //assign given value
            region.assign_advice(|| "assign value", self.value, offset, || value)?;

            Ok(())
        })
    }


}


#[cfg(test)]
mod tests {
    use halo2_proofs::{
        circuit::floor_planner::V1,
        dev::{FailureLocation, MockProver, VerifyFailure},
        pasta::Fp,
        plonk::{Any, Circuit},
    };

    use super::*;

#[derive(Default)]
struct MyCircuit <F: FieldExt, const RANGE: usize> {
    value: Value<Assigned<F>>,
}

    impl<F: FieldExt, const RANGE: usize> Circuit<F> for MyCircuit<F, RANGE> {
        type Config = RangeCheckConfig<F, RANGE>;
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            RangeCheckConfig::configure(meta, value)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            // let chip =RangeCheckChip::construct(config);
            config.assign(layouter.namespace(|| "Assign value"), self.value)?;

            Ok(())
        }
    }

    #[test]
    fn test_range_check() {
        let k = 4;
        const RANGE: usize = 8;

        for i in 0..RANGE {
            let circuit = MyCircuit::<Fp, RANGE> {
                value: Value::known(Fp::from(i as u64).into()),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }
    }
}