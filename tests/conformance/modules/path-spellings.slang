import folder.path_name;import "folder/path-name";float run(){return pathValue();}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
