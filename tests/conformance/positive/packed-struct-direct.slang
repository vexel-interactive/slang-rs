struct S{float a:packoffset(c2.y);float b;};cbuffer Params{float x;S data[2];float y;};RWStructuredBuffer<float> output;[shader("compute")][numthreads(1,1,1)]void main(){output[0]=x+data[1].a+y;}
