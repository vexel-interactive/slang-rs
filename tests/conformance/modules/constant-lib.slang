module constant_lib;

public struct Parameters
{
    public float value;
};

public typealias Uniforms = ConstantBuffer<Parameters>;
public Uniforms parameters;
