struct Params{half scalar;half values[2];half tail;row_major half2x3 rows;half after;};
ConstantBuffer<Params> params;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){output[0]=uint(params.scalar+params.values[1]+params.tail+params.rows[1][2]+params.after);}
