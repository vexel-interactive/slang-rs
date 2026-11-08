// Native ConstantBuffer matrix elements, eight bindings per shader.
ConstantBuffer<int16_t3x1> value31;
ConstantBuffer<int16_t3x2> value32;
ConstantBuffer<int16_t3x3> value33;
ConstantBuffer<int16_t3x4> value34;
ConstantBuffer<int16_t4x1> value41;
ConstantBuffer<int16_t4x2> value42;
ConstantBuffer<int16_t4x3> value43;
ConstantBuffer<int16_t4x4> value44;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){
output[0]=uint(value31[2][0]);
output[1]=uint(value32[2][1]);
output[2]=uint(value33[2][2]);
output[3]=uint(value34[2][3]);
output[4]=uint(value41[3][0]);
output[5]=uint(value42[3][1]);
output[6]=uint(value43[3][2]);
output[7]=uint(value44[3][3]);
}
