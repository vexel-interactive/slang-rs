import lib;
typedef Item Alias;
static int bias = 99;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){Alias a=Alias(3); Item b={5.0};output[0]=uint(total(a));output[1]=uint(total(b));output[2]=uint(bias);}
