use std::marker::PhantomData;


/// This helper checks that the value witnesses in a given cell is within a given range.
/// Depending on the rage, this helper uses either a range-check expression (for small ranges)
/// or a lookup (for larger ranges)
///
///        value     |    q_range_check    |   q_lookup  |  table_value  |
///       ----------------------------------------------------------------
///          v_0     |         1           |      0      |       0       |
///          v_1     |         0           |      1      |       1       |
///

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector},
    poly::Rotation,
};

mod table;

use table::RangeCheckTable;

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE:usize, const LOOKUP_RANGE: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    q_lookup: Selector,
    table: RangeCheckTable<F, LOOKUP_RANGE>,
}


impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> RangeCheckConfig<F, RANGE, LOOKUP_RANGE> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>,
    ) -> Self {
        let q_range_check = meta.selector();

        //toogles the q lookup; simple selectors are eleigible for optimisations
        //simple selectos - no lookup arguments
        // complex selectors - lookup arguments
        //simple selectors collapses combining multiple gates and at the end they can be non binary
        //in llokup we dont want to multiply by wired factor and only by `1`
        let q_lookup = meta.complex_selector();

        //configure a lookup table
        let table = RangeCheckTable::configure(meta);

        let config = Self {
            q_range_check,
            value,
            q_lookup,
            table: table.clone()
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
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };

            //you select one selector and with-selector multiplies each expression with 
            // the selector. so basically abstracts away the selector.
            Constraints::with_selector(q_range_check, [("range_check", range_check(RANGE, value))])
        });

        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            let value = meta.query_advice(value, Rotation::cur());
            
            //lookup API returns a vedtor
            vec![
                (q_lookup * value.square(), table.value)
            ]
        });

        config
    }

    //a lot of overhead in remembering the layout of the template
    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
        range: usize
    ) -> Result<(), Error> {
        assert!(range <= LOOKUP_RANGE);

        if (range < RANGE ) {
            layouter.assign_region(|| "Assign value", |mut region| {
                let offset = 0;
    
                //enable q range check. what is region?
                self.q_range_check.enable(&mut region, offset);
    
                //assign given value
                region.assign_advice(|| "assign value", self.value, offset, || value)?;
    
                Ok(())
            })
        } else {
            layouter.assign_region(|| "Assign value in lookup", |mut region| {
                let offset = 0;

                self.q_lookup.enable(&mut region, offset);

                region.assign_advice(|| "assign value", self.value, offset, || value)?;

                Ok(())
            })
        }
        
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
struct MyCircuit <F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> {
    value: Value<Assigned<F>>,
    large_value: Value<Assigned<F>>,
}

    impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> Circuit<F> for MyCircuit<F, RANGE, LOOKUP_RANGE> {
        type Config = RangeCheckConfig<F, RANGE, LOOKUP_RANGE>;
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

            config.table.load(&mut layouter)?;
            // let chip =RangeCheckChip::construct(config);
            config.assign(layouter.namespace(|| "Assign value"), self.value, RANGE)?;
            config.assign(layouter.namespace(|| "Assign large value"), self.large_value, LOOKUP_RANGE)?;

            Ok(())
        }
    }

    #[test]
    fn test_range_check() {
        //k 
        let k = 9;
        const RANGE: usize = 8;
        const LOOKUP_RANGE: usize = 256;

        for i in 0..RANGE {
            let circuit = MyCircuit::<Fp, RANGE, LOOKUP_RANGE> {
                //value = v in  [v * (v-1) * (v-2) * ... * (v-(RANGE-1)) = 0]
                value: Value::known(Fp::from(i as u64).into()),
                large_value: Value::known(Fp::from((i*i) as u64).into()),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

        // {
        //     let circuit = MyCircuit::<Fp, RANGE> {
        //         value: Value::known(Fp::from(RANGE as u64).into()),
        //     };
        //     let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        //     assert_eq!(
        //         prover.verify(),
        //         Err(vec![VerifyFailure::ConstraintNotSatisfied {
        //             constraint: ((0, "range check").into(), 0, "range check").into(),
        //             location: FailureLocation::InRegion {
        //                 region: (0, "Assign value").into(),
        //                 offset: 0
        //             },
        //             cell_values: vec![(((Any::Advice, 0).into(), 0).into(), "0x8".to_string())]
        //         }])
        //     );
        // }
    }
}