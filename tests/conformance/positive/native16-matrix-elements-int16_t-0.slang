// Native ConstantBuffer matrix elements, eight bindings per shader.
ConstantBuffer<int16_t1x1> value11;
ConstantBuffer<int16_t1x2> value12;
ConstantBuffer<int16_t1x3> value13;
ConstantBuffer<int16_t1x4> value14;
ConstantBuffer<int16_t2x1> value21;
ConstantBuffer<int16_t2x2> value22;
ConstantBuffer<int16_t2x3> value23;
ConstantBuffer<int16_t2x4> value24;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){
output[0]=uint(value11[0][0]);
output[1]=uint(value12[0][1]);
output[2]=uint(value13[0][2]);
output[3]=uint(value14[0][3]);
output[4]=uint(value21[1][0]);
output[5]=uint(value22[1][1]);
output[6]=uint(value23[1][2]);
output[7]=uint(value24[1][3]);
}
