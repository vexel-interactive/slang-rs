interface I { int read(float value); }
struct Item : I { int read(inout float value) { return 11; } int read(int value) { return 17; } };
int run<T:I>(T value) { return value.read(1.0); }
RWStructuredBuffer<uint> output;[shader("compute")][numthreads(1,1,1)]void main(){Item item;output[0]=run(item);}
