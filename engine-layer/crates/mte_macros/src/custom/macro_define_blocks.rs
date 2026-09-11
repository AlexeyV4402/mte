use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitBool, LitStr, Token, parse_macro_input};

struct BlockDefinition {
    name: Ident,
    solid: LitBool,
    shape: Expr,
    profile: Ident,
    textures: Vec<LitStr>,
}

// Реализуем трейт Parse, чтобы syn знал, как читать наш синтаксис:
// Air => { solid: false, shape: Shape::None, profile: AllSides, textures: [...] }
impl Parse for BlockDefinition {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        syn::braced!(content in input);

        // Читаем solid
        content.parse::<Ident>()?; // пропускаем "solid"
        content.parse::<Token![:]>()?;
        let solid: LitBool = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем shape
        content.parse::<Ident>()?; // пропускаем "shape"
        content.parse::<Token![:]>()?;
        let shape: Expr = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем profile
        content.parse::<Ident>()?; // пропускаем "profile"
        content.parse::<Token![:]>()?;
        let profile: Ident = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем textures
        content.parse::<Ident>()?; // пропускаем "textures"
        content.parse::<Token![:]>()?;

        let tex_array;
        syn::bracketed!(tex_array in content);
        let mut textures = Vec::new();
        while !tex_array.is_empty() {
            let tex: LitStr = tex_array.parse()?;
            textures.push(tex);
            if tex_array.is_empty() {
                break;
            }
            tex_array.parse::<Token![,]>()?;
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        Ok(BlockDefinition {
            name,
            solid,
            shape,
            profile,
            textures,
        })
    }
}

struct BlocksList {
    blocks: Vec<BlockDefinition>,
}

impl Parse for BlocksList {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut blocks = Vec::new();
        while !input.is_empty() {
            blocks.push(input.parse()?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(BlocksList { blocks })
    }
}

pub fn define_blocks_impl(input: TokenStream) -> TokenStream {
    let input_list = parse_macro_input!(input as BlocksList);

    let mut enum_variants = Vec::new();
    let mut match_solid = Vec::new();
    let mut match_shape = Vec::new();
    let mut match_profile = Vec::new();
    let mut property_initializers = Vec::new();
    let mut all_textures = Vec::new();

    let mut current_block_id = 0;
    let mut current_texture_index = 0u32;

    for block in input_list.blocks {
        let name = &block.name;
        let solid = &block.solid;
        let shape = &block.shape;
        let profile = &block.profile;

        for tex in &block.textures {
            all_textures.push(tex.value());
        }

        enum_variants.push(quote::quote! { #name });
        match_solid.push(quote::quote! { BlockType::#name => #solid });
        match_shape.push(quote::quote! { BlockType::#name => #shape });
        match_profile.push(quote::quote! { BlockType::#name => TextureMappingProfile::#profile });

        property_initializers.push(quote::quote! {
            BlockProperty { base_id: #current_texture_index, profile_id: TextureMappingProfile::#profile as u32 }
        });

        current_texture_index += block.textures.len() as u32;
        current_block_id += 1;
    }

    let padding_count = 4096 - current_block_id;
    let padding =
        vec![quote::quote! { BlockProperty { base_id: 0, profile_id: 0 } }; padding_count];

    let expanded = quote::quote! {
        pub const REGISTERED_BLOCKS_COUNT: usize = #current_block_id;
        pub const REGISTERED_TEXTURES_COUNT: u32 = #current_texture_index;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum BlockType {
            #( #enum_variants ),*
        }

        impl BlockType {
            #[inline(always)]
            pub fn is_solid(self) -> bool {
                match self {
                    #( #match_solid ),*
                }
            }

            #[inline(always)]
            pub fn is_transparent(self) -> bool {
                !self.is_solid()
            }

            #[inline(always)]
            pub fn get_shape(self) -> Shape {
                match self {
                    #( #match_shape ),*
                }
            }

            #[inline(always)]
            pub fn get_texture_mapping_profile(self) -> TextureMappingProfile {
                match self {
                    #( #match_profile ),*
                }
            }
        }

        pub const BLOCK_PROPERTIES_REGISTRY: [BlockProperty; 4096] = [
            #( #property_initializers, )*
            #( #padding ),*
        ];
    };

    TokenStream::from(expanded)
}
