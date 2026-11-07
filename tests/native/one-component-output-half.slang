StructuredBuffer<float> inputs;
RWStructuredBuffer<float> results;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    half1 input0 = half1(inputs[0]);
    half1 sine0, cosine0, whole0;
    int1 exponent0;
    sincos(input0, sine0, cosine0);
    half1 fraction0 = modf(input0, whole0);
    half1 mantissa0 = frexp(input0, exponent0);
    results[0] = float(sine0[0] + cosine0[0] + fraction0[0] + mantissa0[0]) + float(exponent0[0]);
    half1x1 input1 = half1x1(inputs[1]);
    half1x1 sine1, cosine1, whole1;
    int1x1 exponent1;
    sincos(input1, sine1, cosine1);
    half1x1 fraction1 = modf(input1, whole1);
    half1x1 mantissa1 = frexp(input1, exponent1);
    results[1] = float(sine1[0][0] + cosine1[0][0] + fraction1[0][0] + mantissa1[0][0]) + float(exponent1[0][0]);
    half1x2 input2 = half1x2(inputs[2]);
    half1x2 sine2, cosine2, whole2;
    int1x2 exponent2;
    sincos(input2, sine2, cosine2);
    half1x2 fraction2 = modf(input2, whole2);
    half1x2 mantissa2 = frexp(input2, exponent2);
    results[2] = float(sine2[0][0] + cosine2[0][0] + fraction2[0][0] + mantissa2[0][0]) + float(exponent2[0][0]);
    half1x3 input3 = half1x3(inputs[3]);
    half1x3 sine3, cosine3, whole3;
    int1x3 exponent3;
    sincos(input3, sine3, cosine3);
    half1x3 fraction3 = modf(input3, whole3);
    half1x3 mantissa3 = frexp(input3, exponent3);
    results[3] = float(sine3[0][0] + cosine3[0][0] + fraction3[0][0] + mantissa3[0][0]) + float(exponent3[0][0]);
    half1x4 input4 = half1x4(inputs[4]);
    half1x4 sine4, cosine4, whole4;
    int1x4 exponent4;
    sincos(input4, sine4, cosine4);
    half1x4 fraction4 = modf(input4, whole4);
    half1x4 mantissa4 = frexp(input4, exponent4);
    results[4] = float(sine4[0][0] + cosine4[0][0] + fraction4[0][0] + mantissa4[0][0]) + float(exponent4[0][0]);
    half2x1 input5 = half2x1(inputs[5]);
    half2x1 sine5, cosine5, whole5;
    int2x1 exponent5;
    sincos(input5, sine5, cosine5);
    half2x1 fraction5 = modf(input5, whole5);
    half2x1 mantissa5 = frexp(input5, exponent5);
    results[5] = float(sine5[0][0] + cosine5[0][0] + fraction5[0][0] + mantissa5[0][0]) + float(exponent5[0][0]);
    half3x1 input6 = half3x1(inputs[6]);
    half3x1 sine6, cosine6, whole6;
    int3x1 exponent6;
    sincos(input6, sine6, cosine6);
    half3x1 fraction6 = modf(input6, whole6);
    half3x1 mantissa6 = frexp(input6, exponent6);
    results[6] = float(sine6[0][0] + cosine6[0][0] + fraction6[0][0] + mantissa6[0][0]) + float(exponent6[0][0]);
    half4x1 input7 = half4x1(inputs[7]);
    half4x1 sine7, cosine7, whole7;
    int4x1 exponent7;
    sincos(input7, sine7, cosine7);
    half4x1 fraction7 = modf(input7, whole7);
    half4x1 mantissa7 = frexp(input7, exponent7);
    results[7] = float(sine7[0][0] + cosine7[0][0] + fraction7[0][0] + mantissa7[0][0]) + float(exponent7[0][0]);
}
