struct A{int value;};typealias Alias=A;
interface I { int read(A value,float factor); }
struct Item : I { int read(Alias value,int factor) { return value.value+factor; } };
int run<T:I>(T value) { return value.read(A(2),1.0); }
RWStructuredBuffer<uint> output;[shader("compute")][numthreads(1,1,1)]void main(){Item item;output[0]=run(item);}
