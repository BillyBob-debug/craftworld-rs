// Authored bounded adapter to pinned MaterialBox/MaterialWire readers.
// CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
// MaterialBox/MaterialWire/MaterialParameterAnimation::serialize, MemoryInputStream::s32.
// Layout/provenance/unknown semantics: docs/material-graph.md. No graph execution.
import cwlib.io.streams.MemoryInputStream;
import cwlib.io.serializer.Serializer;
import cwlib.types.data.Revision;
import cwlib.structs.gmat.MaterialBox;
import cwlib.structs.gmat.MaterialWire;
import cwlib.structs.gmat.MaterialParameterAnimation;
import java.io.*;
import java.nio.*;
import java.util.*;

public class MaterialGraphOracle {
    static Object lastDiffuseRoute;
    static Object lastStaticMultiply;
    static void bounds(MemoryInputStream s)throws IOException {
        if(s.getOffset()>s.getBuffer().length)throw new IOException("padded/truncated graph field");
    }
    static List<Long> vectorBits(MemoryInputStream s)throws IOException {
        List<Long> v=new ArrayList<>();for(int i=0;i<4;i++)v.add(MaterialOracle.unsigned(s.i32(true)));bounds(s);return v;
    }
    static Map<String,Object> animation(MemoryInputStream s)throws Exception {
        List<Long> base=vectorBits(s);int count=MaterialOracle.count(s,4096);
        byte[] data=s.bytes(Math.multiplyExact(count,4));bounds(s);
        int nameLength=MaterialOracle.count(s,64);String name=HexFormat.of().formatHex(s.bytes(nameLength));bounds(s);
        int mask=s.i8()&255,components=Integer.bitCount(mask&15);
        if(count!=0&&(components==0||count%components!=0))throw new IOException("animation component count/mask");
        return TextureOracle.row("base_value_bits",base,"component_value_count",count,
            "component_bytes_sha256",TextureOracle.sha(data),"name_hex",name,"components_raw",mask,
            "derived_key_count",count==0?0:count/components);
    }
    static void compareAnimation(MaterialParameterAnimation a,Map<String,Object> expected)throws Exception {
        ByteArrayOutputStream bytes=new ByteArrayOutputStream();
        if(a.keys!=null)for(int component=0;component<4;component++)if((a.componentsAnimated&(1<<component))!=0)
            for(var key:a.keys)bytes.write(ByteBuffer.allocate(4).putInt(Float.floatToRawIntBits(key.get(component))).array());
        if(!TextureOracle.sha(bytes.toByteArray()).equals(expected.get("component_bytes_sha256")) ||
            (a.componentsAnimated&255)!=(int)expected.get("components_raw"))throw new IOException("animation component oracle disagreement");
        List<Long> bits=new ArrayList<>();for(int i=0;i<4;i++)bits.add(MaterialOracle.unsigned(Float.floatToRawIntBits(a.baseValue.get(i))));
        if(!bits.equals(expected.get("base_value_bits")))throw new IOException("animation base oracle disagreement");
        String name=new String(HexFormat.of().parseHex((String)expected.get("name_hex"))).replace("\0", "");
        if(!a.getName().equals(name))throw new IOException("animation name oracle disagreement");
    }
    static Map<String,Object> graph(MemoryInputStream s,Revision revision)throws Exception {
        if(revision.getVersion()<920||revision.getVersion()>951||revision.getSubVersion()!=0||revision.isVita()||revision.isToolkit())
            throw new IOException("unsupported graph revision");
        cwlib.resources.RGfxMaterial routeMaterial=new cwlib.resources.RGfxMaterial();
        int start=s.getOffset(),boxCount=MaterialOracle.count(s,1024);List<Object> boxes=new ArrayList<>();
        for(int i=0;i<boxCount;i++) {
            int begin=s.getOffset();long type=MaterialOracle.unsigned(s.i32());
            int declared=MaterialOracle.count(s,8);if(declared!=8)throw new IOException("unsupported declared parameter count");
            List<Long> params=new ArrayList<>();for(int j=0;j<8;j++)params.add(MaterialOracle.unsigned(s.i32()));
            List<Long> position=vectorBits(s);long subtype=MaterialOracle.unsigned(s.i32());
            Map<String,Object> first=animation(s),second=animation(s);bounds(s);
            MemoryInputStream isolated=new MemoryInputStream(Arrays.copyOfRange(s.getBuffer(),begin,s.getOffset()),s.getCompressionFlags());
            MaterialBox box=new MaterialBox();box.serialize(new Serializer(isolated,revision));
            if(isolated.getOffset()!=isolated.getBuffer().length||MaterialOracle.unsigned(box.type)!=type||MaterialOracle.unsigned(box.subType)!=subtype)
                throw new IOException("box reader boundary/type disagreement");
            for(int j=0;j<8;j++)if(MaterialOracle.unsigned(box.getParameters()[j])!=params.get(j))throw new IOException("box parameter disagreement");
            float[] xywh={box.x,box.y,box.w,box.h};
            for(int j=0;j<4;j++)if(MaterialOracle.unsigned(Float.floatToRawIntBits(xywh[j]))!=position.get(j))throw new IOException("box position disagreement");
            if((box.anim.keys==null?0:box.anim.keys.length)!=(int)first.get("derived_key_count") || (box.anim2.keys==null?0:box.anim2.keys.length)!=(int)second.get("derived_key_count"))
                throw new IOException("animation key count disagreement");
            compareAnimation(box.anim,first);compareAnimation(box.anim2,second);
            routeMaterial.boxes.add(box);
            Long slot=type==1?params.get(5):null;
            boxes.add(TextureOracle.row("type_raw",type,"declared_parameter_count",declared,"parameters_raw",params,
                "position_bits",position,"subtype_raw",subtype,"animations",List.of(first,second),
                "texture_slot_raw",slot,"texture_slot_in_range",slot==null?null:slot<8));
        }
        int wireCount=MaterialOracle.count(s,4096);List<Object> wires=new ArrayList<>();
        for(int i=0;i<wireCount;i++) {
            int begin=s.getOffset(),from=s.s32(),to=s.s32(),portFrom=s.i8()&255,portTo=s.i8()&255;
            List<Integer> swizzle=new ArrayList<>();for(int j=0;j<5;j++)swizzle.add(s.i8()&255);bounds(s);
            MemoryInputStream isolated=new MemoryInputStream(Arrays.copyOfRange(s.getBuffer(),begin,s.getOffset()),s.getCompressionFlags());
            MaterialWire wire=new MaterialWire();wire.serialize(new Serializer(isolated,revision));
            if(isolated.getOffset()!=isolated.getBuffer().length||wire.boxFrom!=from||wire.boxTo!=to||(wire.portFrom&255)!=portFrom||(wire.portTo&255)!=portTo)
                throw new IOException("wire reader disagreement");
            for(int j=0;j<5;j++)if((wire.swizzle[j]&255)!=swizzle.get(j))throw new IOException("wire swizzle disagreement");
            routeMaterial.wires.add(wire);
            wires.add(TextureOracle.row("box_from",from,"box_to",to,"port_from_raw",portFrom,"port_to_raw",portTo,
                "swizzle_raw",swizzle,"from_in_range",from>=0&&from<boxCount,"to_in_range",to>=0&&to<boxCount));
        }
        bounds(s);
        int output=routeMaterial.getOutputBox();
        MaterialWire diffuse=output<0?null:routeMaterial.getWireConnectedToPort(output,0);
        MaterialBox source=diffuse==null?null:routeMaterial.getBoxConnectedToPort(output,0);
        lastDiffuseRoute=TextureOracle.row("output_index",output,"source_index",diffuse==null?null:diffuse.boxFrom,"source_type",source==null?null:MaterialOracle.unsigned(source.type),"slot",source!=null&&source.type==1?MaterialOracle.unsigned(source.getParameters()[5]):null,"uv_channel_raw",source!=null&&source.type==1?MaterialOracle.unsigned(source.getParameters()[4]):null);
        // Authored external helper query; no shader assembly or copied product policy.
        lastStaticMultiply=null;
        if(source!=null&&source.type==10) {
            MaterialWire left=routeMaterial.getWireConnectedToPort(diffuse.boxFrom,0);
            MaterialWire right=routeMaterial.getWireConnectedToPort(diffuse.boxFrom,1);
            if(left!=null&&right!=null) {
                MaterialBox a=routeMaterial.boxes.get(left.boxFrom),b=routeMaterial.boxes.get(right.boxFrom);
                MaterialWire texture=a.type==1&&b.type==3?left:a.type==3&&b.type==1?right:null;
                MaterialWire color=a.type==1&&b.type==3?right:a.type==3&&b.type==1?left:null;
                if(texture!=null) {
                    List<Long> bits=new ArrayList<>();
                    for(int i=0;i<4;i++)bits.add(MaterialOracle.unsigned(routeMaterial.boxes.get(color.boxFrom).getParameters()[i]));
                    lastStaticMultiply=TextureOracle.row("multiply_index",diffuse.boxFrom,"sample_index",texture.boxFrom,"color_index",color.boxFrom,"tint_bits",bits);
                }
            }
        }
        return TextureOracle.row("box_count",boxCount,"wire_count",wireCount,"boxes",boxes,"wires",wires,
            "graph_start",start,"parsed_graph_bytes",s.getOffset()-start,"unparsed_bytes",s.getBuffer().length-s.getOffset());
    }
    static void integer(ByteArrayOutputStream b,int n,boolean compact)throws Exception {
        if(!compact){b.write(ByteBuffer.allocate(4).putInt(n).array());return;}
        long value=Integer.toUnsignedLong(n);do{int part=(int)(value&127);value>>>=7;b.write(part|(value!=0?128:0));}while(value!=0);
    }
    static byte[] recipe(boolean compact)throws Exception {
        ByteArrayOutputStream b=new ByteArrayOutputStream();integer(b,1,compact);integer(b,1,compact);integer(b,8,compact);
        for(int j=0;j<8;j++)integer(b,j==5?7:0,compact);
        b.write(new byte[16]);integer(b,0,compact);
        for(int j=0;j<2;j++){b.write(new byte[16]);integer(b,0,compact);integer(b,3,compact);b.write(new byte[3]);b.write(0);}
        integer(b,1,compact);integer(b,compact?1:-1,compact);integer(b,0,compact);b.write(new byte[]{0,(byte)255,0,1,2,3,4});
        return b.toByteArray();
    }
    static void selfTest()throws Exception {
        for(boolean compact:new boolean[]{false,true}) {
            byte[] bytes=recipe(compact);Map<String,Object> result=graph(new MemoryInputStream(bytes,(byte)(compact?7:0)),new Revision(950));
            if(!result.get("box_count").equals(1)||!result.get("wire_count").equals(1))throw new AssertionError("graph counts");
            Map<?,?> wire=(Map<?,?>)((List<?>)result.get("wires")).get(0);
            if(!wire.get("box_from").equals(-1)||!wire.get("from_in_range").equals(false))throw new AssertionError("signed/null endpoint");
            for(int n=0;n<bytes.length;n++) {
                try {graph(new MemoryInputStream(Arrays.copyOf(bytes,n),(byte)(compact?7:0)),new Revision(950));}
                catch(Exception rejected){continue;}
                throw new AssertionError("accepted graph truncation "+n);
            }
        }
        // Toolkit s32 uses arithmetic >> after an unsigned value is cast to int.
        // Record its extreme ZigZag conflict; never treat it as correct signed decoding.
        int extreme=new MemoryInputStream(new byte[]{(byte)255,(byte)255,(byte)255,(byte)255,15},(byte)7).s32();
        if(extreme!=0)throw new AssertionError("recorded extreme signed oracle behavior changed");
        System.out.println("Known Toolkit s32 conflict: ZigZag 0xffffffff returns "+extreme+" instead of Integer.MIN_VALUE");
        byte[] invalid=recipe(false);ByteBuffer.wrap(invalid).putInt(8,7);
        try{graph(new MemoryInputStream(invalid),new Revision(950));throw new AssertionError("declared count accepted");}catch(IOException rejected){}
        invalid=ByteBuffer.allocate(4).putInt(1025).array();
        try{graph(new MemoryInputStream(invalid),new Revision(950));throw new AssertionError("box budget accepted");}catch(IOException rejected){}
        invalid=ByteBuffer.allocate(29).putInt(16,1).array();
        try{animation(new MemoryInputStream(invalid));throw new AssertionError("component/mask accepted");}catch(IOException rejected){}
        System.out.println("Synthetic material graph fixed/packed/signed/truncation guards passed");
    }
}
