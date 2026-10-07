// Authored bounded prefix adapter invoking pinned Toolkit stream/position readers.
// Pin and per-rule provenance: docs/mesh-prefix.md. No third-party source translated.
// Full RMesh deserialization/topology is intentionally not invoked.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.io.streams.MemoryInputStream;
import cwlib.resources.RMesh;
import cwlib.structs.mesh.Bone;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;
import java.security.MessageDigest;
public class MeshOracle {
    static long u(int n){return Integer.toUnsignedLong(n);}
    static int count(MemoryInputStream s,int max)throws IOException{int n=s.i32();if(n<0||n>max)throw new IOException("count budget");return n;}
    static List<Long> floats(MemoryInputStream s)throws IOException{int n=count(s,16);List<Long> a=new ArrayList<>();for(int i=0;i<n;i++)a.add(u(s.i32(true)));return a;}
    static byte[] array(MemoryInputStream s)throws IOException{return s.bytes(count(s,16*1024*1024));}
    static Map<String,Object> descriptor(MemoryInputStream s,String key)throws IOException{
        long flags=u(s.i32());int kind=s.i8()&255;if((kind&~3)!=0)throw new IOException("descriptor kind");
        Long guid=(kind&2)!=0?u(s.i32()):null;String sha=(kind&1)!=0?HexFormat.of().formatHex(s.bytes(20)):null;
        return TextureOracle.row(key+"_flags",flags,key+"_kind",kind,key+"_guid",guid,key+"_hash",sha);
    }
    static void skipBone(MemoryInputStream s)throws IOException{
        s.bytes(32);for(int i=0;i<5;i++)s.i32();s.m44();s.m44();s.bytes(32);
        int n=count(s,4096);for(int i=0;i<n;i++){s.bytes(32);s.i32();}
        n=count(s,4096);for(int i=0;i<n;i++){s.i32();s.i32();}s.bytes(56);
    }
    static String hash(byte[] b,String algorithm)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance(algorithm).digest(b));}
    static void uvSynthetic()throws Exception{
        byte[] positions=new byte[32];
        float[] values={0.25f,-0.0f,4f,5f,0.75f,1f,6f,7f};
        ByteBuffer attributes=ByteBuffer.allocate(32);for(float v:values)attributes.putFloat(v);
        RMesh mesh=new RMesh(new byte[][]{positions},attributes.array(),null,new Bone[0]);
        for(int c=0;c<2;c++){
            var coords=mesh.getUVs(c);
            for(int i=0;i<2;i++){
                if(Float.floatToRawIntBits(coords[i].x)!=Float.floatToRawIntBits(values[i*4+c*2])||
                    Float.floatToRawIntBits(coords[i].y)!=Float.floatToRawIntBits(values[i*4+c*2+1]))
                    throw new IOException("synthetic interleaved UV disagreement");
            }
        }
        // Nonzero start is a byte seek in this pinned method; whole-channel getters avoid it.
        if(Float.floatToRawIntBits(mesh.getUVs(1,1,0)[0].x)==Float.floatToRawIntBits(0.75f))
            throw new IOException("pinned nonzero-start behavior changed");
        System.out.println("UV synthetic channels/negative-zero and pinned nonzero-start conflict verified");
    }
    public static void main(String[] args)throws Exception{
        if(args.length!=3&&args.length!=4)throw new IllegalArgumentException("FARC MANIFEST NEW-JSON [--uv | --groups]");
        boolean uv=args.length==4&&args[3].equals("--uv");
        boolean groups=args.length==4&&args[3].equals("--groups");
        if(args.length==4&&!uv&&!groups)throw new IllegalArgumentException("unknown option");
        if(uv) uvSynthetic();
        Path out=Path.of(args[2]).toAbsolutePath();
        if(Files.exists(out)||!out.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored output required");
        JsonArray samples=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject().getAsJsonArray("samples");
        if(samples.size()>48)throw new IOException("sample count budget");
        List<Object> results=new ArrayList<>();long total=0;
        try(RandomAccessFile f=new RandomAccessFile(args[0],"r")){
            for(JsonElement item:samples){JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();Map<String,Object> row=TextureOracle.row("sha1",sha);results.add(row);
                try{
                    long offset=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(offset<0||size<18||size>2*1024*1024||offset>f.length()-size||total+size>64*1024*1024)throw new IOException("source budget");
                    byte[] raw=new byte[(int)size];f.seek(offset);f.readFully(raw);total+=size;
                    if(!hash(raw,"SHA-1").equals(sha))throw new IOException("payload identifier mismatch");
                    if(!Arrays.equals(Arrays.copyOf(raw,4),new byte[]{'M','S','H','b'}))throw new IOException("only clear MSHb");
                    ByteBuffer header=ByteBuffer.wrap(raw);int rev=header.getInt(4),end=header.getInt(8),start=12,flags=0;
                    if(rev<0x189||rev>1016||end<18||end>raw.length-4)throw new IOException("header range/revision");
                    int branch=0,branchRevision=0;if(rev>=0x271){branch=Short.toUnsignedInt(header.getShort(start));branchRevision=Short.toUnsignedInt(header.getShort(start+2));start+=4;}
                    if(branch!=0&&!(rev==0x272&&branch==0x4c44))throw new IOException("branch unsupported");
                    if(rev>=0x297||(rev==0x272&&branch==0x4c44&&branchRevision>=0x2)){flags=raw[start++]&255;}
                    boolean compressed=raw[start++]!=0;
                    if(flags!=0&&flags!=7)throw new IOException("flags unsupported");
                    int dependencies=header.getInt(end);if(dependencies<0||dependencies>4096)throw new IOException("dependency budget");
                    byte[] payload=Arrays.copyOfRange(raw,start,end);
                    byte[] clear=payload;
                    if(compressed){if(payload.length<4||TextureOracle.be16(payload,0)>1)throw new IOException("chunk flag");
                        byte[] fake=new byte[4+payload.length];System.arraycopy(new byte[]{'T','E','X',' '},0,fake,0,4);System.arraycopy(payload,0,fake,4,payload.length);clear=TextureOracle.clearPayload(fake);}
                    if(clear.length>16*1024*1024)throw new IOException("expanded budget");
                    SerializedResource resource=new SerializedResource(raw);
                    if(resource.getCompressionFlags()!=flags||!Arrays.equals(clear,resource.getStream().getBuffer()))throw new IOException("wrapper disagreement");
                    MemoryInputStream s=resource.getStream();int[] counts=new int[7];for(int i=0;i<7;i++)counts[i]=count(s,i<4?1000000:34);
                    if(counts[0]>100000||counts[4]<1||counts[4]>34||counts[5]>8||counts[6]>32)throw new IOException("mesh prefix budget");
                    List<String> names=new ArrayList<>();for(int i=0;i<32;i++)names.add(HexFormat.of().formatHex(s.bytes(16)));
                    List<Long> minUV=new ArrayList<>(),maxUV=new ArrayList<>();Long area=null;
                    if(rev>=0x239){minUV=floats(s);maxUV=floats(s);area=u(s.i32(true));}
                    List<Long> offsets=new ArrayList<>();for(int i=0;i<counts[4]+2;i++)offsets.add(u(s.i32()));
                    long streamSize=(long)counts[0]*16,streamBytes=streamSize*counts[4];if(streamBytes>16*1024*1024)throw new IOException("stream budget");
                    byte[][] streams=new byte[counts[4]][];for(int i=0;i<streams.length;i++)streams[i]=s.bytes((int)streamSize);
                    byte[] attributes=array(s),indices=array(s);
                    RMesh mesh=new RMesh(new byte[][]{streams[0]},null,null,new Bone[0]);
                    var vertices=mesh.getVertices();float[][] bounds=null;MessageDigest positions=MessageDigest.getInstance("SHA-256");
                    for(int i=0;i<vertices.length;i++){var v=vertices[i];float[] xyz={v.x,v.y,v.z};
                        if(!Float.isFinite(v.x)||!Float.isFinite(v.y)||!Float.isFinite(v.z))throw new IOException("non-finite positions");
                        if(bounds==null)bounds=new float[][]{xyz.clone(),xyz.clone()};
                        for(int j=0;j<3;j++){bounds[0][j]=Math.min(bounds[0][j],xyz[j]);bounds[1][j]=Math.max(bounds[1][j],xyz[j]);}
                        positions.update(streams[0],i*16,12);
                    }
                    long[][] boundsBits=null;
                    if(bounds!=null){boundsBits=new long[2][3];for(int i=0;i<2;i++)for(int j=0;j<3;j++)boundsBits[i][j]=u(Float.floatToRawIntBits(bounds[i][j]));}
                    row.putAll(TextureOracle.row("status","parsed_prefix","prefix",TextureOracle.row(
                        "revision",rev,"compression_flags",flags,"counts",counts,"morph_names_hex",names,
                        "min_uv_bits",minUV,"max_uv_bits",maxUV,"area_scale_bits",area,"stream_offsets",offsets,
                        "position_bounds",bounds,"position_bounds_bits",boundsBits,"position_bytes_sha256",HexFormat.of().formatHex(positions.digest()),
                        "attribute_bytes",attributes.length,"attribute_sha256",hash(attributes,"SHA-256"),
                        "index_bytes",indices.length,"index_sha256",hash(indices,"SHA-256"),
                        "expanded_bytes",clear.length,"parsed_prefix_bytes",s.getOffset(),"unparsed_bytes",clear.length-s.getOffset())));
                    if(uv){
                        if(attributes.length!=(long)counts[0]*counts[5]*8)throw new IOException("UV count/attribute extent conflict");
                        List<Object> channels=new ArrayList<>();
                        if(counts[5]>0){
                            RMesh uvMesh=new RMesh(new byte[][]{streams[0]},attributes,null,new Bone[0]);
                            if(uvMesh.getAttributeCount()!=counts[5])throw new IOException("UV header/constructor disagreement");
                            for(int channel=0;channel<counts[5];channel++){
                                MessageDigest digest=MessageDigest.getInstance("SHA-256");
                                var coords=uvMesh.getUVs(channel);
                                if(coords.length!=counts[0])throw new IOException("UV vertex count disagreement");
                                for(var value:coords){
                                    if(!Float.isFinite(value.x)||!Float.isFinite(value.y))throw new IOException("nonfinite UV");
                                    digest.update(ByteBuffer.allocate(8).putInt(Float.floatToRawIntBits(value.x)).putInt(Float.floatToRawIntBits(value.y)).array());
                                }
                                channels.add(TextureOracle.row("channel",channel,"uv_bits_sha256",HexFormat.of().formatHex(digest.digest())));
                            }
                        }
                        row.put("uv",TextureOracle.row("vertex_count",counts[0],"channel_count",counts[5],"channels",channels));
                    }
                    Object prefix=row.get("prefix");
                    int primitiveCount=count(s,4096);List<Object> primitives=new ArrayList<>();
                    List<int[]> ranges=new ArrayList<>();
                    for(int i=0;i<primitiveCount;i++){
                        Map<String,Object> primitive=descriptor(s,"material");primitive.putAll(descriptor(s,"alternatives"));
                        long min=u(s.i32()),max=u(s.i32());int first=s.i32(),length=s.i32();long region=u(s.i32());
                        if(first<0||length<0||(long)first+length>counts[1])throw new IOException("primitive range");
                        primitive.putAll(TextureOracle.row("min_vertex_raw",min,"max_vertex_raw",max,"first_index",first,"index_count",length,"region",region));
                        primitives.add(primitive);ranges.add(new int[]{first,length});
                    }
                    int boneCount=count(s,4096);for(int i=0;i<boneCount;i++)skipBone(s);
                    s.bytes(count(s,4096)*2);s.bytes(count(s,4096));s.bytes(count(s,32)*2);
                    int topology=s.i8()&255;if(topology!=5&&topology!=6)throw new IOException("unsupported topology");
                    RMesh geometry=new RMesh(new byte[][]{streams[0]},null,indices,new Bone[0],counts[1],counts[2],counts[3]);
                    geometry.setPrimitiveType(cwlib.enums.CellGcmPrimitive.fromValue(topology));
                    MessageDigest triangles=MessageDigest.getInstance("SHA-256");int triangleCount=0;
                    List<Object> partitions=new ArrayList<>();
                    for(int[] range:ranges){
                        int[] faces=range[1]<3?new int[0]:geometry.getTriangles(range[0],range[1]);
                        MessageDigest partition=MessageDigest.getInstance("SHA-256");
                        int firstTriangle=triangleCount;
                        if(faces.length%3!=0)throw new IOException("oracle triangle list length");
                        triangleCount+=faces.length/3;for(int index:faces){if(index<0||index>=counts[0])throw new IOException("oracle vertex range");
                            byte[] encoded=ByteBuffer.allocate(4).putInt(index).array();
                            triangles.update(encoded);partition.update(encoded);}
                        partitions.add(TextureOracle.row("start",firstTriangle,"end",triangleCount,"triangle_indices_sha256",HexFormat.of().formatHex(partition.digest())));
                    }
                    if(groups) row.put("groups",partitions);
                    row.put("geometry",TextureOracle.row("prefix",prefix,"primitives",primitives,"bone_count",boneCount,
                        "primitive_type",topology,"triangle_count",triangleCount,"triangle_indices_sha256",HexFormat.of().formatHex(triangles.digest()),
                        "parsed_geometry_bytes",s.getOffset(),"unparsed_after_geometry",clear.length-s.getOffset()));
                }catch(Exception e){row.put("status","failed");row.put("reason",e.getClass().getSimpleName()+": "+e.getMessage());}
            }
        }
        try(Writer w=Files.newBufferedWriter(out,StandardOpenOption.CREATE_NEW)){new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(TextureOracle.row("toolkit_commit","d6a2ef80b5a09999f08f23a803b516096055e5c9","archived_bytes_read",total,"samples",results),w);}
        System.out.printf("Mesh prefix oracle: %d samples, %d archived bytes%n",results.size(),total);
    }
}
