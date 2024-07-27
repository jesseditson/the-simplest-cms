// AUTO-GENERATED by typescript-type-def

export type I32 = number;
export type Usize = number;
export type ValuePathComponent = ({
    "Key": string;
} | {
    "Index": Usize;
});
export type ValuePath = {
    "path": (ValuePathComponent)[];
};
export type F64 = number;
export type DateTime = {
    "raw": string;
};
export type File = {
    "sha": string;
    "name": (string | null);
    "filename": string;
    "mime": string;
    "display_type": string;
    "url": string;
};
export type MetaValue = ({
    "String": string;
} | {
    "Number": F64;
} | {
    "Boolean": boolean;
} | {
    "DateTime": DateTime;
} | {
    "Array": MetaValue[];
} | {
    "Map": Record<string, MetaValue>;
});
export type Meta = Record<string, MetaValue>;
export type FieldValue = ("None" | {
    "String": string;
} | {
    "Markdown": string;
} | {
    "Number": F64;
} | {
    "Date": DateTime;
} | {
    "Objects": Record<string, FieldValue>[];
} | {
    "Boolean": boolean;
} | {
    "File": File;
} | {
    "Meta": Meta;
});
export type AddObjectValue = {
    "path": ValuePath;
    "value": FieldValue;
};
export type AddObjectEvent = {
    "object": string;
    "filename": string;
    "order": I32;
    "values": (AddObjectValue)[];
};
export type DeleteObjectEvent = {
    "object": string;
    "filename": string;
};
export type EditFieldEvent = {
    "object": string;
    "filename": string;
    "path": ValuePath;
    "field": string;
    "value": FieldValue;
};
export type EditOrderEvent = {
    "object": string;
    "filename": string;
    "order": I32;
};
export type ChildEvent = {
    "object": string;
    "filename": string;
    "path": ValuePath;
};
export type ArchivalEvent = ({
    "AddObject": AddObjectEvent;
} | {
    "DeleteObject": DeleteObjectEvent;
} | {
    "EditField": EditFieldEvent;
} | {
    "EditOrder": EditOrderEvent;
} | {
    "AddChild": ChildEvent;
} | {
    "RemoveChild": ChildEvent;
});
