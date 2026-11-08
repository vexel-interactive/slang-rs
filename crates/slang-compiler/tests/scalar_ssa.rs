use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_ir::flow::{
    Definition, Lattice, Operation, Terminator, ValueId, build_function, propagate,
};

fn compile(source: &str) -> Compilation {
    compile_source(
        "ssa.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn code(source: &str) -> String {
    compile(source).artifact.unwrap()
}

#[test]
fn scalar_compound_shifts_saturate_and_indexed_destinations_keep_one_evaluation() {
    let compiled = code(include_str!(
        "../../../tests/conformance/positive/integer16-compound-shifts.slang"
    ));
    for text in [
        "(x = int16_t(0))",
        "(y = uint16_t(0u))",
        "(z = int16_t(-1))",
    ] {
        assert!(compiled.contains(text), "{text}: {compiled}");
    }
    assert_eq!(compiled.matches("i++").count(), 1, "{compiled}");
    assert!(compiled.contains("<<= 16"), "{compiled}");
}

#[test]
fn same_values_merge_across_dynamic_branches_but_different_values_stay_dynamic() {
    let same = code("int f(bool flag){int x=1;if(flag)x=7;else x=7;return x+2;}");
    assert!(same.contains("return 9;"), "{same}");
    let different = code("int f(bool flag){int x=1;if(flag)x=7;else x=8;return x+2;}");
    assert!(different.contains("return (x + 2);"), "{different}");
}

#[test]
fn unreachable_branches_do_not_poison_constants_and_return_paths_do_not_merge() {
    let compiled =
        code("int f(int input){int x=3;if(false)x=input;if(input>0)return input;x<<=32;return x;}");
    assert!(compiled.contains("(x = 0)"), "{compiled}");
    assert!(compiled.contains("return 0;"), "{compiled}");
}

#[test]
fn loop_backedges_make_changing_values_dynamic_but_preserve_invariants() {
    let compiled = code("int f(int count){int i=0,x=7;while(i<count){i++;x=7;}return x+i;}");
    assert!(compiled.contains("while ((i < count))"), "{compiled}");
    assert!(compiled.contains("return (7 + i);"), "{compiled}");
}

#[test]
fn zero_iteration_and_single_iteration_loops_have_distinct_reaching_values() {
    let compiled = code("int f(){int x=1;while(false)x=3;do{x+=4;}while(false);return x;}");
    assert!(compiled.contains("return 5;"), "{compiled}");
}

#[test]
fn break_continue_and_switch_fallthrough_preserve_executable_edges() {
    let compiled = code(
        "int f(){int x=1;for(int i=0;i<4;i++){switch(i){case 0:x+=2;break;default:continue;}break;}switch(1){case 0:x=9;break;case 1:x+=3;case 2:x+=4;break;default:x=99;}return x;}",
    );
    assert!(compiled.contains("return 10;"), "{compiled}");
}

#[test]
fn short_circuit_and_conditional_side_effects_only_flow_from_selected_arms() {
    let compiled = code(
        "int f(){int x=1;bool a=false&&((x=9)>0);bool b=true||((x=8)>0);int y=true?(x+=2):(x+=4);return x+y+int(a)+int(b);}",
    );
    assert!(compiled.contains("return 7;"), "{compiled}");
    assert!(compiled.contains("x = 3"), "{compiled}");
}

#[test]
fn compound_inputs_are_read_after_right_hand_side_writes_and_keep_side_effects() {
    let compiled = code("int f(){int x=1;x+=(x=3);x+=x++;return x;}");
    assert!(compiled.contains("(x = ((x = 3) , 6))"), "{compiled}");
    assert!(compiled.contains("(x = ((x++) , 13))"), "{compiled}");
    assert!(compiled.contains("return 13;"), "{compiled}");
    assert_eq!(compiled.matches("x++").count(), 1, "{compiled}");
}

#[test]
fn output_parameters_invalidate_local_values_without_rewriting_locations() {
    let compiled = code(
        "void change(inout int x){x+=2;}void set(out uint x){x=9;}uint f(){int x=3;uint y=0;change(x);set(y);return uint(x)+y;}",
    );
    assert!(compiled.contains("change(x);"), "{compiled}");
    assert!(compiled.contains("set(y);"), "{compiled}");
    assert!(compiled.contains("uint(x) + y"), "{compiled}");
}

#[test]
fn initialized_intrinsic_outputs_remain_writable_and_dynamic() {
    let compiled = code("uint f(double input){uint lo=0,hi=0;asuint(input,lo,hi);return lo+hi;}");
    assert!(compiled.contains("asuint(input, lo, hi)"), "{compiled}");
    assert!(compiled.contains("return (lo + hi);"), "{compiled}");
}

#[test]
fn lexical_shadows_and_uninitialized_locals_have_distinct_ssa_values() {
    let compiled =
        code("int f(int input){int x=2;{int x=input;x+=3;}return x;}int g(){int x;return x;}");
    assert!(compiled.contains("return 2;"), "{compiled}");
    assert!(compiled.contains("return x;"), "{compiled}");
}

#[test]
fn integer_widths_and_pointer_types_keep_wrapping_and_signed_shifts() {
    let compiled = code(
        "uint64_t f(){uint64_t x=0xffffffffffffffffULL;x/=3ULL;return x;}int64_t g(){int64_t x=-1LL;x>>=64;return x;}uint h(){uint16_t x=uint16_t(65535);x+=uint16_t(3);return uint(x);}intptr_t j(){intptr_t x=intptr_t(-1);x>>=64;return x;}",
    );
    for text in [
        "return 6148914691236517205ull;",
        "return -1ll;",
        "return 2u;",
    ] {
        assert!(compiled.contains(text), "{text}: {compiled}");
    }
}

#[test]
fn analysis_graph_has_verified_block_arguments_and_use_def_links() {
    let compiled = compile("int f(int count){int i=0,x=7;while(i<count){i++;x=7;}return x+i;}");
    let graph = build_function(&compiled.ir, 0).unwrap();
    graph.verify().unwrap();
    assert!(graph.blocks.iter().any(|b| !b.parameters.is_empty()));
    assert!(
        graph
            .values
            .iter()
            .any(|v| matches!(v.definition, Definition::Parameter { .. }))
    );
    let uses = graph.use_blocks();
    for (index, block) in graph.blocks.iter().enumerate() {
        for value in block
            .instructions
            .iter()
            .flat_map(|i| i.operation.operands())
            .chain(block.terminator.operands())
        {
            assert!(uses[value.0].contains(&slang_ir::flow::BlockId(index)));
        }
    }
    let propagation = propagate(&graph);
    assert!(propagation.values.contains(&Lattice::Constant(7)));
    assert!(propagation.values.contains(&Lattice::Any));
}

#[test]
fn verifier_rejects_missing_operands_wrong_edge_arguments_and_nondominating_values() {
    let compiled = compile("int f(bool flag,int input){int x;if(flag)x=input;else x=2;return x;}");
    let graph = build_function(&compiled.ir, 0).unwrap();
    graph.verify().unwrap();
    let mut missing = graph.clone();
    missing.blocks[0].instructions[0].operation = Operation::Copy(ValueId(usize::MAX));
    assert!(missing.verify().unwrap_err().contains("missing value"));
    let mut wrong_edge = graph.clone();
    let branch = wrong_edge
        .blocks
        .iter_mut()
        .find_map(|b| {
            if let Terminator::Branch { on_true, .. } = &mut b.terminator {
                Some(on_true)
            } else {
                None
            }
        })
        .unwrap();
    branch.arguments.clear();
    assert!(wrong_edge.verify().unwrap_err().contains("argument count"));
    let mut dominance = graph.clone();
    let (yes, no) = graph
        .blocks
        .iter()
        .find_map(|b| {
            if let Terminator::Branch {
                on_true, on_false, ..
            } = &b.terminator
            {
                Some((on_true.target, on_false.target))
            } else {
                None
            }
        })
        .unwrap();
    let foreign = graph.blocks[yes.0].instructions[0].result;
    dominance.blocks[no.0].instructions[0].operation = Operation::Copy(foreign);
    assert!(dominance.verify().unwrap_err().contains("dominate"));
    let mut ordering = graph.clone();
    let forward = ordering.blocks[0].instructions[2].result;
    ordering.blocks[0].instructions[1].operation = Operation::Copy(forward);
    assert!(ordering.verify().unwrap_err().contains("before"));
}

#[test]
fn oversized_optional_analysis_skips_optimization_without_rejecting_compilation() {
    let mut source = "int f(){".to_owned();
    for index in 0..200 {
        source.push_str(&format!("int value{index}={index};"));
    }
    source.push_str("return value199;}");
    let compiled = compile(&source);
    assert!(build_function(&compiled.ir, 0).is_none());
    assert!(compiled.artifact.unwrap().contains("return value199;"));
    assert!(build_function(&compiled.ir, usize::MAX).is_none());
}
