RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){int value=1;value<<=2.5;output[0]=uint(value);}
