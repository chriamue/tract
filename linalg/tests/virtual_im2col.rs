use proptest::arbitrary::Arbitrary;
use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy};
use tract_data::internal::*;
use tract_linalg::frame::mmm::FusedSpec;
use tract_linalg::frame::mmm::{VirtualInput, VirtualInputSpec};
use DatumType::F32;

proptest::proptest! {
    #[test]
    fn prop(pb in any::<ConvProblem>()) {
        pb.check()
    }
}

#[test]
fn test1() {
    ConvProblem { input: tensor3(&[[[1f32]]]), filters: tensor4(&[[[[-1f32]]]]) }.check()
}

#[test]
fn test2() {
    ConvProblem { input: tensor3(&[[[0f32], [-1.0]]]), filters: tensor4(&[[[[0f32, -1f32]]]]) }.check()
}

// CHW HWIO CHW 
// 121 1112 221 

// 2D valid, no group, no dil, no stride, HWIO, CHW
#[derive(Clone, Debug)]
struct ConvProblem {
    input: Tensor,
    filters: Tensor,
}

fn mknhw(filters: &[usize], input: &[usize]) -> (usize, usize, usize, usize, usize) {
    let m = filters[3];
    let k = filters[0..3].iter().product::<usize>();
    let h = input[1] - filters[0] + 1;
    let w = input[2] - filters[1] + 1;
    let n = h * w;
    (m, k, n, h, w)
}

impl ConvProblem {
    fn reference(&self) -> Tensor {
        let (m, _, _, h, w) = mknhw(self.filters.shape(), self.input.shape());
        let output_shape = [m, h, w];
        let mut output = Tensor::zero::<f32>(&output_shape).unwrap();
        let mut output_view = output.to_array_view_mut::<f32>().unwrap();
        let input_view = self.input.to_array_view::<f32>().unwrap();
        let filters_view = self.filters.to_array_view::<f32>().unwrap();
        for geo_out in tract_ndarray::indices(&output_shape[1..]) {
            for ker_geo in tract_ndarray::indices(&self.filters.shape()[0..2]) {
                for ci in 0..self.filters.shape()[2] {
                    for co in 0..self.filters.shape()[3] {
                        let output_coord = [co, geo_out[0], geo_out[1]];
                        let input_coord = [ci, geo_out[0] + ker_geo[0], geo_out[1] + ker_geo[1]];
                        let ker_coord = [ker_geo[0], ker_geo[1], ci, co];
                        output_view[output_coord] +=
                            filters_view[ker_coord] * input_view[input_coord];
                    }
                }
            }
        }
        output
    }

    fn tract(&self) -> Tensor {
        let (m, k, n, h, w) = mknhw(self.filters.shape(), self.input.shape());
        let output_shape = [m, h, w];
        let mut output = Tensor::zero::<f32>(&output_shape).unwrap();
        let mmm = tract_linalg::generic().mmm(F32, F32, F32, Some(m), Some(k), Some(n)).unwrap();
        let mut output = Tensor::zero::<f32>(&output_shape).unwrap();
        let mut packed_filter =
            Tensor::zero_aligned::<f32>(&[mmm.a_pack().len(k, m)], mmm.a_pack().alignment())
                .unwrap();
        let reshaped_filters = self.filters.clone().into_shape(&[k, m]).unwrap();
        unsafe {
            mmm.a_pack().pack(packed_filter.view_mut(), reshaped_filters.view(), 0, 1);
            dbg!(&packed_filter);
            let a_store = mmm.a_packed(F32.size_of(), k).wrap(&packed_filter.view());
            let im2col = EagerIm2colSpec { full_kernel_shape: self.filters.shape().into() };
            let b_store =
                mmm.b_virtual_input(Box::new(im2col), k).wrap(&self.input.view()).unwrap();
            let c_store = mmm.c_view().wrap(&mut output.view());
            mmm.run(
                m,
                n,
                &[FusedSpec::AddMatMul { k, a: a_store, b: b_store }, FusedSpec::Store(c_store)],
            )
            .unwrap()
        }
        output
    }

    fn check(&self) {
        let found = self.tract();
        let expected = self.reference();
        if found.close_enough(&expected, true).is_err() {
            println!("found: ");
            println!("{:?}", found.to_array_view::<f32>().unwrap());
            println!("expected: ");
            println!("{:?}", expected.to_array_view::<f32>().unwrap());
        }
        found.close_enough(&expected, true).unwrap()
    }
}

impl Arbitrary for ConvProblem {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;
    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        (1..4usize, 1..4usize, 1..4usize, 1..4usize, 0..3usize, 0..3usize)
            .prop_flat_map(|(h, w, i, o, extra_h, extra_w)| {
                let filters = tensor(vec![h, w, i, o]);
                let input = tensor(vec![i, h + extra_h, w + extra_w]);
                (filters, input)
            })
            .prop_map(|(filters, input)| ConvProblem { filters, input })
            .boxed()
    }
}

fn tensor(shape: Vec<usize>) -> BoxedStrategy<Tensor> {
    let len = shape.iter().product::<usize>();
    proptest::collection::vec(any::<i8>(), len..=len)
        .prop_map(move |vec| {
            tract_ndarray::ArrayD::from_shape_vec(shape.clone(), vec)
                .unwrap()
                .into_tensor()
                .cast_to_dt(F32)
                .unwrap()
                .into_owned()
        })
        .boxed()
}

#[derive(Clone, Debug, Hash)]
struct EagerIm2colSpec {
    full_kernel_shape: TVec<usize>,
}

impl_dyn_hash!(EagerIm2colSpec);

impl VirtualInputSpec for EagerIm2colSpec {
    fn wrap(&self, input: &TensorView) -> Box<dyn VirtualInput> {
        let (_, k, n, h, w) = mknhw(&self.full_kernel_shape, input.shape());
        // let input = input.to_array_view::<f32>().unwrap();
        let ci = input.shape()[0];
        let kh = self.full_kernel_shape[0];
        let kw = self.full_kernel_shape[1];
        let output = tract_ndarray::Array5::<f32>::from_shape_fn(
            [kh, kw, ci, h, w],
            |(kh, kw, ci, h, w)| *input.at([ci, h + kh, w + kw]).unwrap(),
        ).into_shape([k, n]).unwrap();
        dbg!(&output);
        Box::new(EagerIm2col { im2col: output.into_tensor() })
    }
}

#[derive(Clone, Debug, Hash)]
struct EagerIm2col {
    im2col: Tensor,
}
impl_dyn_hash!(EagerIm2col);

impl VirtualInput for EagerIm2col {
    fn input(
        &self,
        packer: &tract_linalg::frame::Packer,
        packed: *mut u8,
        k_range: std::ops::Range<usize>,
        mn_range: std::ops::Range<usize>,
    ) {
        let mn = self.im2col.shape()[1];
        unsafe {
            packer.pack_t::<f32>(
                packed as _,
                self.im2col.as_ptr().unwrap(),
                mn,
                mn as isize,
                1,
                k_range,
                mn_range,
            );
            dbg!(std::slice::from_raw_parts(packed as *const f32, 4));
        }
    }
}