[shader("fragment")]
float4 main(float input : TEXCOORD0) : SV_Target
{
    float total = 0.0;
    double1 value0 = double1(input);
    double1 derivative0 = ddx(value0) + ddy_fine(value0) + fwidth_coarse(value0);
    total += float(derivative0[0]);
    double1x1 value1 = double1x1(input);
    double1x1 derivative1 = ddx(value1) + ddy_fine(value1) + fwidth_coarse(value1);
    total += float(derivative1[0][0]);
    double1x2 value2 = double1x2(input);
    double1x2 derivative2 = ddx(value2) + ddy_fine(value2) + fwidth_coarse(value2);
    total += float(derivative2[0][0]);
    double1x3 value3 = double1x3(input);
    double1x3 derivative3 = ddx(value3) + ddy_fine(value3) + fwidth_coarse(value3);
    total += float(derivative3[0][0]);
    double1x4 value4 = double1x4(input);
    double1x4 derivative4 = ddx(value4) + ddy_fine(value4) + fwidth_coarse(value4);
    total += float(derivative4[0][0]);
    double2x1 value5 = double2x1(input);
    double2x1 derivative5 = ddx(value5) + ddy_fine(value5) + fwidth_coarse(value5);
    total += float(derivative5[0][0]);
    double3x1 value6 = double3x1(input);
    double3x1 derivative6 = ddx(value6) + ddy_fine(value6) + fwidth_coarse(value6);
    total += float(derivative6[0][0]);
    double4x1 value7 = double4x1(input);
    double4x1 derivative7 = ddx(value7) + ddy_fine(value7) + fwidth_coarse(value7);
    total += float(derivative7[0][0]);
    return float4(total);
}
