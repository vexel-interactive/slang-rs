import lib;float run(){Pair p;p.value=twice(1);return p.value+data[0];}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
