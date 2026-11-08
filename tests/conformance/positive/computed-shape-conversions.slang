ConstantBuffer<vector<float,true>> single;
ConstantBuffer<vector<float,int(3.5)>> input;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    vector<float,3.5> value = vector<float,(1<2?3:2)>(1,2,3);
    vector<float,(3==2?2:3)> other = input;
    output[0] = single.x + input.z + value.y + other.x;
}
