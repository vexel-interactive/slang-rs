import lib;float run(){float _slang_module_1_value=3;return value+_slang_module_1_value;}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
