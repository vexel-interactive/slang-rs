// Native component storage, both matrix orders, all 16 shapes.
struct Params{
    uint16_t lead;
    column_major uint16_t1x1 m11;
    column_major uint16_t1x2 m12;
    column_major uint16_t1x3 m13;
    column_major uint16_t1x4 m14;
    column_major uint16_t2x1 m21;
    column_major uint16_t2x2 m22;
    column_major uint16_t2x3 m23;
    column_major uint16_t2x4 m24;
    column_major uint16_t3x1 m31;
    column_major uint16_t3x2 m32;
    column_major uint16_t3x3 m33;
    column_major uint16_t3x4 m34;
    column_major uint16_t4x1 m41;
    column_major uint16_t4x2 m42;
    column_major uint16_t4x3 m43;
    column_major uint16_t4x4 m44;
    uint16_t tail;
};
ConstantBuffer<Params> params;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){output[0]=uint(params.lead+params.m11[0][0]+params.m12[0][1]+params.m13[0][2]+params.m14[0][3]+params.m21[1][0]+params.m22[1][1]+params.m23[1][2]+params.m24[1][3]+params.m31[2][0]+params.m32[2][1]+params.m33[2][2]+params.m34[2][3]+params.m41[3][0]+params.m42[3][1]+params.m43[3][2]+params.m44[3][3]+params.tail);}
