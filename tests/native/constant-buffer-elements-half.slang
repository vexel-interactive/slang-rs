// Native value loading and legacy DXC layout for every numeric element shape.
ConstantBuffer<half> input0 : register(b0);
ConstantBuffer<half1> input1 : register(b1);
ConstantBuffer<half2> input2 : register(b2);
ConstantBuffer<half3> input3 : register(b3);
ConstantBuffer<half4> input4 : register(b4);
ConstantBuffer<half1x1> input5 : register(b5);
ConstantBuffer<half1x2> input6 : register(b6);
ConstantBuffer<half1x3> input7 : register(b7);
ConstantBuffer<half1x4> input8 : register(b8);
ConstantBuffer<half2x1> input9 : register(b9);
ConstantBuffer<half2x2> input10 : register(b10);
ConstantBuffer<half2x3> input11 : register(b11);
ConstantBuffer<half2x4> input12 : register(b12);
ConstantBuffer<half3x1> input13 : register(b13);
ConstantBuffer<half3x2> input14 : register(b14);
ConstantBuffer<half3x3> input15 : register(b15);
ConstantBuffer<half3x4> input16 : register(b16);
ConstantBuffer<half4x1> input17 : register(b17);
ConstantBuffer<half4x2> input18 : register(b18);
ConstantBuffer<half4x3> input19 : register(b19);
ConstantBuffer<half4x4> input20 : register(b20);
RWStructuredBuffer<float> output;
half load0(ConstantBuffer<half> value) { return value; }
half1 load1(ConstantBuffer<half1> value) { return value; }
half2 load2(ConstantBuffer<half2> value) { return value; }
half3 load3(ConstantBuffer<half3> value) { return value; }
half4 load4(ConstantBuffer<half4> value) { return value; }
half1x1 load5(ConstantBuffer<half1x1> value) { return value; }
half1x2 load6(ConstantBuffer<half1x2> value) { return value; }
half1x3 load7(ConstantBuffer<half1x3> value) { return value; }
half1x4 load8(ConstantBuffer<half1x4> value) { return value; }
half2x1 load9(ConstantBuffer<half2x1> value) { return value; }
half2x2 load10(ConstantBuffer<half2x2> value) { return value; }
half2x3 load11(ConstantBuffer<half2x3> value) { return value; }
half2x4 load12(ConstantBuffer<half2x4> value) { return value; }
half3x1 load13(ConstantBuffer<half3x1> value) { return value; }
half3x2 load14(ConstantBuffer<half3x2> value) { return value; }
half3x3 load15(ConstantBuffer<half3x3> value) { return value; }
half3x4 load16(ConstantBuffer<half3x4> value) { return value; }
half4x1 load17(ConstantBuffer<half4x1> value) { return value; }
half4x2 load18(ConstantBuffer<half4x2> value) { return value; }
half4x3 load19(ConstantBuffer<half4x3> value) { return value; }
half4x4 load20(ConstantBuffer<half4x4> value) { return value; }
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
