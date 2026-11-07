module lib;
public struct Pair {public float value;float hidden;};
float helper(float x){return x*2.0;}
public float twice(float x){return helper(x);}
public float select(float x){return x+1.0;}
int select(int x){return x+2;}
public float innerSelect(){return float(select(1));}
public StructuredBuffer<float> data;
public static const float value=2;
static float secret=3;
