import lib;float run(){return select(1)+innerSelect();}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
