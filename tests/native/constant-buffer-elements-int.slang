// Native value loading and legacy DXC layout for every numeric element shape.
ConstantBuffer<int> input0 : register(b0);
ConstantBuffer<int1> input1 : register(b1);
ConstantBuffer<int2> input2 : register(b2);
ConstantBuffer<int3> input3 : register(b3);
ConstantBuffer<int4> input4 : register(b4);
ConstantBuffer<int1x1> input5 : register(b5);
ConstantBuffer<int1x2> input6 : register(b6);
ConstantBuffer<int1x3> input7 : register(b7);
ConstantBuffer<int1x4> input8 : register(b8);
ConstantBuffer<int2x1> input9 : register(b9);
ConstantBuffer<int2x2> input10 : register(b10);
ConstantBuffer<int2x3> input11 : register(b11);
ConstantBuffer<int2x4> input12 : register(b12);
ConstantBuffer<int3x1> input13 : register(b13);
ConstantBuffer<int3x2> input14 : register(b14);
ConstantBuffer<int3x3> input15 : register(b15);
ConstantBuffer<int3x4> input16 : register(b16);
ConstantBuffer<int4x1> input17 : register(b17);
ConstantBuffer<int4x2> input18 : register(b18);
ConstantBuffer<int4x3> input19 : register(b19);
ConstantBuffer<int4x4> input20 : register(b20);
RWStructuredBuffer<float> output;
int load0(ConstantBuffer<int> value) { return value; }
int1 load1(ConstantBuffer<int1> value) { return value; }
int2 load2(ConstantBuffer<int2> value) { return value; }
int3 load3(ConstantBuffer<int3> value) { return value; }
int4 load4(ConstantBuffer<int4> value) { return value; }
int1x1 load5(ConstantBuffer<int1x1> value) { return value; }
int1x2 load6(ConstantBuffer<int1x2> value) { return value; }
int1x3 load7(ConstantBuffer<int1x3> value) { return value; }
int1x4 load8(ConstantBuffer<int1x4> value) { return value; }
int2x1 load9(ConstantBuffer<int2x1> value) { return value; }
int2x2 load10(ConstantBuffer<int2x2> value) { return value; }
int2x3 load11(ConstantBuffer<int2x3> value) { return value; }
int2x4 load12(ConstantBuffer<int2x4> value) { return value; }
int3x1 load13(ConstantBuffer<int3x1> value) { return value; }
int3x2 load14(ConstantBuffer<int3x2> value) { return value; }
int3x3 load15(ConstantBuffer<int3x3> value) { return value; }
int3x4 load16(ConstantBuffer<int3x4> value) { return value; }
int4x1 load17(ConstantBuffer<int4x1> value) { return value; }
int4x2 load18(ConstantBuffer<int4x2> value) { return value; }
int4x3 load19(ConstantBuffer<int4x3> value) { return value; }
int4x4 load20(ConstantBuffer<int4x4> value) { return value; }
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float value = 0;
    value += float(load0(input0));
    value += float(load1(input1)[0]);
    value += float(load2(input2)[0]);
    value += float(load3(input3)[0]);
    value += float(load4(input4)[0]);
    value += float(load5(input5)[0][0]);
    value += float(load6(input6)[0][0]);
    value += float(load7(input7)[0][0]);
    value += float(load8(input8)[0][0]);
    value += float(load9(input9)[0][0]);
    value += float(load10(input10)[0][0]);
    value += float(load11(input11)[0][0]);
    value += float(load12(input12)[0][0]);
    value += float(load13(input13)[0][0]);
    value += float(load14(input14)[0][0]);
    value += float(load15(input15)[0][0]);
    value += float(load16(input16)[0][0]);
    value += float(load17(input17)[0][0]);
    value += float(load18(input18)[0][0]);
    value += float(load19(input19)[0][0]);
    value += float(load20(input20)[0][0]);
    output[0] = value;
}
