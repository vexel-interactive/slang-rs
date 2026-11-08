// Native half storage with explicit packing and logical value reconstruction.
struct Params{half anchor:packoffset(c1);half value;half2 pair;};
ConstantBuffer<Params> params;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){output[0]=uint(params.anchor+params.value+params.pair.x+params.pair.y);}
