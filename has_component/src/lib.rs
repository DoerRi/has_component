use std::{any::Any, marker::PhantomData};

pub use has_component_derive::HasComponent;
use tuple_info::TupleInfo;

/// The trait users interact with.
pub trait HasComponent {
    fn get_component<C: 'static>(&self) -> Option<&C> {
        None
    }
    fn get_mut_component<C: 'static>(&mut self) -> Option<&mut C> {
        None
    }

    fn component_types() -> Vec<std::any::TypeId>;

    fn get_components<'a, C: 'static + TupleInfo>(
        &'a self,
    ) -> Option<<C as TupleInfo>::DeconstructedReference<'a>>
    where
        Self: Sized,
    {
        // Define Startegy to extract component references which the tuple has as anies from self
        struct Strategy<T> {
            _t: PhantomData<T>,
        }
        impl<'a, SELF: HasComponent + 'a> tuple_info::ForeachTypeStrategie<'a> for Strategy<SELF> {
            type Output = Option<&'a dyn Any>;

            type Input = &'a SELF;

            fn action<T: 'static>(
                input: Self::Input,
                _type_index: usize,
            ) -> (Self::Output, Self::Input) {
                (input.get_component::<T>().map(|t| t as &dyn Any), input)
            }
        }

        // apply strategy
        let anies = C::foreach_type::<Strategy<Self>>(self)
            .0
            .into_iter()
            .collect::<Option<Vec<&dyn Any>>>()?;
        // construct deconstruction reference
        C::try_deconstruction(&anies)
    }

    fn component<C: 'static>(&self) -> &C {
        match self.get_component::<C>() {
            Some(component) => component,
            _ => panic!(
                "This Actor doesn't have the component {}",
                std::any::type_name::<C>()
            ),
        }
    }
    fn mut_component<C: 'static>(&mut self) -> &mut C {
        match self.get_mut_component::<C>() {
            Some(component) => component,
            _ => panic!(
                "This Actor doesn't have the component {}",
                std::any::type_name::<C>()
            ),
        }
    }
    fn get_mut_components<'a, C: TupleInfo>(
        &'a mut self,
    ) -> Option<<C as TupleInfo>::MutDeconstructedReference<'a>>;
}

pub fn reorder_components<'a, const LEN: usize>(
    mut components: [Option<&'a mut dyn Any>; LEN],
    type_order: &[std::any::TypeId],
) -> Vec<&'a mut dyn Any> {
    const NONE: Option<&mut dyn Any> = None;
    let mut result: [Option<&'a mut dyn Any>; LEN] = [NONE; LEN];

    for (i, &tid) in type_order.iter().enumerate() {
        // find the first component whose std::any::TypeId matches
        if let Some(pos) = components
            .iter()
            .position(|c| c.as_ref().map(|c| (**c).type_id()) == Some(tid))
        {
            // take it out of the original array and put it into result[i]
            result[i] = components[pos].take();
        } else {
            // if any type is missing, fail
            return vec![];
        }
    }
    result
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .unwrap_or(vec![])
}
//pub trait HasComponents<'a, C: 'static + TupleInfo> {
//    fn _get_components(&'a self) -> Option<<C as TupleInfo>::DeconstructedReference<'a>>;
//}
/// Implementation Example
/// ----------------------
/// ```
/// impl<'a, A: 'static, B: 'static, X: HasComponent> HasComponents<'a, (A, B)> for X {
///    fn _get_components(&'a self) -> Option<<(A, B) as TupleInfo>::DeconstructedReference<'a>> {
///        Some((self.get_component::<A>()?, self.get_component::<B>()?))
///    }
/// }
/// ```
//macro_rules! impl_has_components {
//    ($($T:ident),+) => {
//        impl<'a, $($T: 'static,)+ X: HasComponent> HasComponents<'a, ($($T,)+)> for X {
//            fn _get_components(&'a self) -> Option<<($($T,)+) as TupleInfo>::DeconstructedReference<'a>> {
//                Some((
//                    $( self.get_component::<$T>()?, )+
//                ))
//            }
//        }
//    };
//}
//impl_has_components!(A, B);
//impl_has_components!(A, B, C);
//impl_has_components!(A, B, C, D);
//impl_has_components!(A, B, C, D, E);
//impl_has_components!(A, B, C, D, E, F);
//impl_has_components!(A, B, C, D, E, F, G);
//impl_has_components!(A, B, C, D, E, F, G, H);
//impl_has_components!(A, B, C, D, E, F, G, H, I);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K, L);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K, L, M);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
//impl_has_components!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

#[cfg(test)]
mod tests {

    use super::HasComponent;
    #[derive(HasComponent)]
    struct SampleBundle(usize, f32, String);
    #[test]
    fn test_SampleBundle() {
        let mut sample = SampleBundle(123, 456.0, "Hallo".to_string());
        //let x = sample.get_components::<(usize, f32)>();
        //assert_eq!(x, Some((&sample.0, &sample.1)));
        let mut_x = sample.get_mut_components::<(usize, f32)>();

        let x = [1, 2, 3];
    }

    struct Transform {
        pub x: f32,
    }

    struct Velocity {
        pub dx: f32,
    }

    struct AttackDamage {
        pub damage: f32,
    }

    //#[derive(HasComponent)]
    struct Entity {
        transform: Transform,
        velocity: Velocity,
        attack_damage: AttackDamage,
    }
}
