struct Other {int x;};
interface I { static int read(This receiver, float value); }
struct Item : I { int read(int value) { return 11; } static int read(Other receiver, float value) { return 17; } };
int run<T:I>(T value) { return T.read(value,1.0); }
RWStructuredBuffer<uint> output;[shader("compute")][numthreads(1,1,1)]void main(){Item item;output[0]=run(item);}
