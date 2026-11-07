import lib;float parameter(float value){return value;}float run(){float total=value;{float value=total;total=value;}for(int value=0;value<2;value++){total+=float(value);}return parameter(total)+value;}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
