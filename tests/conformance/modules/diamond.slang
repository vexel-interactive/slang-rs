import left;import right;import left;float run(){return leftValue()+rightValue();}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
