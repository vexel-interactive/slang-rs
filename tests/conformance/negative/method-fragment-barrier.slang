struct S { int value; void sync() { GroupMemoryBarrier(); } };
[shader("fragment")]float4 main():SV_Target { S x={1}; x.sync(); return float4(1); }
