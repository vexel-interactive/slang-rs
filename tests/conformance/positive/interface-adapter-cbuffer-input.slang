struct A { int value; };
ConstantBuffer<A> input;
interface I { int read(ConstantBuffer<A> value,float factor); }
struct Item:I { int read(A value,int factor) {return value.value+factor;} };
int run<T:I>(T item){return item.read(input,1.0);}
RWStructuredBuffer<uint> output;[shader("compute")][numthreads(1,1,1)]void main(){Item item;output[0]=run(item);}
