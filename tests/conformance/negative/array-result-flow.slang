float[2] values(bool flag) { if (flag) return {1, 2}; }
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1, 1, 1)]void main(uint3 id : SV_DispatchThreadID) { result[0] = values(id.x != 0)[0]; }
