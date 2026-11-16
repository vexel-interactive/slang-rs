struct A{int value;}; struct B{int value;};
interface I { A read(float value); }
struct Item : I { B read(float value) { return {13}; } A read(int value) { return {17}; } };
int run<T:I>(T value) { return value.read(1.0).value; }
RWStructuredBuffer<uint> output;[shader("compute")][numthreads(1,1,1)]void main(){Item item;output[0]=run(item);}
