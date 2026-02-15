import * as SelectPrimitive from "@radix-ui/react-select";
import { Check, ChevronDown, ChevronUp } from "lucide-react";
import type * as React from "react";

import { cn } from "../../lib/utils";

const Select = SelectPrimitive.Root;

const SelectGroup = SelectPrimitive.Group;

const SelectValue = SelectPrimitive.Value;

interface SelectTriggerProps
	extends React.ComponentPropsWithoutRef<typeof SelectPrimitive.Trigger> {
	ref?: React.Ref<React.ElementRef<typeof SelectPrimitive.Trigger>>;
}

function SelectTrigger({
	className,
	children,
	ref,
	...props
}: SelectTriggerProps) {
	return (
		<SelectPrimitive.Trigger
			ref={ref}
			className={cn(
				"flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
				className,
			)}
			{...props}
		>
			{children}
			<SelectPrimitive.Icon asChild>
				<ChevronDown className="h-4 w-4 opacity-50" />
			</SelectPrimitive.Icon>
		</SelectPrimitive.Trigger>
	);
}
SelectTrigger.displayName = SelectPrimitive.Trigger.displayName;

interface SelectContentProps
	extends React.ComponentPropsWithoutRef<typeof SelectPrimitive.Content> {
	ref?: React.Ref<React.ElementRef<typeof SelectPrimitive.Content>>;
}

function SelectContent({
	className,
	children,
	position = "popper",
	ref,
	...props
}: SelectContentProps) {
	return (
		<SelectPrimitive.Portal>
			<SelectPrimitive.Content
				ref={ref}
				className={cn(
					"relative z-50 min-w-[8rem] overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-md animate-in fade-in-80",
					position === "popper" && "translate-y-1",
					className,
				)}
				position={position}
				{...props}
			>
				<SelectPrimitive.ScrollUpButton className="flex cursor-default items-center justify-center py-1">
					<ChevronUp className="h-4 w-4" />
				</SelectPrimitive.ScrollUpButton>
				<SelectPrimitive.Viewport className="p-1">
					{children}
				</SelectPrimitive.Viewport>
				<SelectPrimitive.ScrollDownButton className="flex cursor-default items-center justify-center py-1">
					<ChevronDown className="h-4 w-4" />
				</SelectPrimitive.ScrollDownButton>
			</SelectPrimitive.Content>
		</SelectPrimitive.Portal>
	);
}
SelectContent.displayName = SelectPrimitive.Content.displayName;

interface SelectLabelProps
	extends React.ComponentPropsWithoutRef<typeof SelectPrimitive.Label> {
	ref?: React.Ref<React.ElementRef<typeof SelectPrimitive.Label>>;
}

function SelectLabel({ className, ref, ...props }: SelectLabelProps) {
	return (
		<SelectPrimitive.Label
			ref={ref}
			className={cn("px-2 py-1.5 text-sm font-semibold", className)}
			{...props}
		/>
	);
}
SelectLabel.displayName = SelectPrimitive.Label.displayName;

interface SelectItemProps
	extends React.ComponentPropsWithoutRef<typeof SelectPrimitive.Item> {
	ref?: React.Ref<React.ElementRef<typeof SelectPrimitive.Item>>;
}

function SelectItem({ className, children, ref, ...props }: SelectItemProps) {
	return (
		<SelectPrimitive.Item
			ref={ref}
			className={cn(
				"relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-2 pr-8 text-sm outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
				className,
			)}
			{...props}
		>
			<span className="absolute right-2 flex h-3.5 w-3.5 items-center justify-center">
				<SelectPrimitive.ItemIndicator>
					<Check className="h-4 w-4" />
				</SelectPrimitive.ItemIndicator>
			</span>
			<SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
		</SelectPrimitive.Item>
	);
}
SelectItem.displayName = SelectPrimitive.Item.displayName;

interface SelectSeparatorProps
	extends React.ComponentPropsWithoutRef<typeof SelectPrimitive.Separator> {
	ref?: React.Ref<React.ElementRef<typeof SelectPrimitive.Separator>>;
}

function SelectSeparator({ className, ref, ...props }: SelectSeparatorProps) {
	return (
		<SelectPrimitive.Separator
			ref={ref}
			className={cn("-mx-1 my-1 h-px bg-muted", className)}
			{...props}
		/>
	);
}
SelectSeparator.displayName = SelectPrimitive.Separator.displayName;

export {
	Select,
	SelectGroup,
	SelectValue,
	SelectTrigger,
	SelectContent,
	SelectLabel,
	SelectItem,
	SelectSeparator,
};
